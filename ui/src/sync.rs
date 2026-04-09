use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use iced::Subscription;
use iced::futures::SinkExt;
use iced::stream;
use printcountpay_core::{PrinterId, PrinterRecord};
use ron::de::from_str;
use ron::ser::to_string;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{MissedTickBehavior, sleep, timeout};

use crate::app::{
    ManualPricingBill, ManualPricingBillTombstone, ManualPricingWorkspace, PricingSettings,
    RecordingSession, SnmpPollStatus,
};

pub const SYNC_PORT: u16 = 32_161;
pub const SYNC_DISCOVERY_PORT: u16 = 32_162;
pub const SYNC_FLUSH_INTERVAL: Duration = Duration::from_millis(350);

const DISCOVERY_MAGIC: &str = "printcountpay-sync-discover/v1";
const DISCOVERY_RESPONSE: &str = "printcountpay-sync-master/v1";
const DISCOVERY_ATTEMPTS: usize = 2;
const DISCOVERY_WAIT: Duration = Duration::from_millis(450);
const ROLE_RETRY_DELAY: Duration = Duration::from_secs(1);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const MASTER_STALE_TIMEOUT: Duration = Duration::from_secs(15);
const SYNC_TARGET: &str = "sync";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PollStateEntry {
    pub(crate) printer_id: PrinterId,
    pub(crate) state: SnmpPollStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecordingSessionEntry {
    pub(crate) printer_id: PrinterId,
    pub(crate) session: RecordingSession,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SharedState {
    pub(crate) revision: u64,
    pub(crate) printers: Vec<PrinterRecord>,
    pub(crate) poll_states: Vec<PollStateEntry>,
    pub(crate) recording_sessions: Vec<RecordingSessionEntry>,
    pub(crate) pricing: PricingSettings,
    #[serde(default)]
    pub(crate) bill_sync_supported: bool,
    #[serde(default)]
    pub(crate) manual_bills: Vec<ManualPricingBill>,
    #[serde(default)]
    pub(crate) manual_bill_tombstones: Vec<ManualPricingBillTombstone>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            revision: 1,
            printers: Vec::new(),
            poll_states: Vec::new(),
            recording_sessions: Vec::new(),
            pricing: PricingSettings::default(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PricingSyncPayload {
    pub(crate) id: String,
    pub(crate) pricing: PricingSettings,
    pub(crate) workspace: ManualPricingWorkspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncRole {
    Searching,
    Master,
    Client,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyncStatus {
    pub(crate) role: SyncRole,
    pub(crate) detail: String,
}

#[derive(Debug, Clone)]
pub(crate) enum SyncCommand {
    SetSnapshot(SharedState),
    RequestPoll(PrinterId),
    SyncPrices(PricingSyncPayload),
}

#[derive(Debug, Clone)]
pub(crate) enum SyncEvent {
    Ready(UnboundedSender<SyncCommand>),
    StatusChanged(SyncStatus),
    SnapshotReceived(SharedState),
    PollRequested(PrinterId),
    PricingSyncReceived(PricingSyncPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireMessage {
    Snapshot(SharedState),
    PollRequest(PrinterId),
    PricingSync(PricingSyncPayload),
    Heartbeat,
}

#[derive(Debug)]
enum MasterEvent {
    Accepted(TcpStream, SocketAddr),
    ClientMessage(u64, WireMessage),
    ClientClosed(u64),
}

#[derive(Debug)]
enum ClientEvent {
    Message(WireMessage),
    Disconnected(String),
}

pub(crate) fn subscription() -> Subscription<SyncEvent> {
    Subscription::run(sync_worker)
}

fn sync_worker() -> impl iced::futures::Stream<Item = SyncEvent> {
    stream::channel(100, async |mut output| {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<SyncCommand>();
        let mut latest_snapshot = None::<SharedState>;
        let mut latest_pricing_sync = None::<PricingSyncPayload>;
        let mut last_status = None::<SyncStatus>;

        let _ = output.send(SyncEvent::Ready(command_tx)).await;

        loop {
            if emit_status(
                &mut output,
                &mut last_status,
                SyncStatus {
                    role: SyncRole::Searching,
                    detail: format!(
                        "Searching for sync host on UDP {} / TCP {}.",
                        SYNC_DISCOVERY_PORT, SYNC_PORT
                    ),
                },
            )
            .await
            .is_err()
            {
                break;
            }

            match discover_master().await {
                Ok(Some(master_addr)) => {
                    tracing::info!(target: SYNC_TARGET, "Discovered sync host at {master_addr}");
                    if run_as_client(
                        master_addr,
                        &mut output,
                        &mut command_rx,
                        &mut latest_snapshot,
                        &mut latest_pricing_sync,
                        &mut last_status,
                    )
                    .await
                    .is_err()
                    {
                        sleep(ROLE_RETRY_DELAY).await;
                    }
                }
                Ok(None) => match MasterSockets::bind().await {
                    Ok(master) => {
                        tracing::info!(
                            target: SYNC_TARGET,
                            "No sync host found, becoming host on TCP {} / UDP {}",
                            SYNC_PORT,
                            SYNC_DISCOVERY_PORT
                        );
                        if run_as_master(
                            master,
                            &mut output,
                            &mut command_rx,
                            &mut latest_snapshot,
                            &mut latest_pricing_sync,
                            &mut last_status,
                        )
                        .await
                        .is_err()
                        {
                            sleep(ROLE_RETRY_DELAY).await;
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: SYNC_TARGET,
                            "Failed to bind sync host sockets: {}",
                            error
                        );
                        sleep(ROLE_RETRY_DELAY).await;
                    }
                },
                Err(error) => {
                    tracing::warn!(
                        target: SYNC_TARGET,
                        "Sync discovery failed: {}",
                        error
                    );
                    sleep(ROLE_RETRY_DELAY).await;
                }
            }
        }
    })
}

async fn run_as_master(
    sockets: MasterSockets,
    output: &mut (impl iced::futures::Sink<SyncEvent> + Unpin),
    command_rx: &mut mpsc::UnboundedReceiver<SyncCommand>,
    latest_snapshot: &mut Option<SharedState>,
    latest_pricing_sync: &mut Option<PricingSyncPayload>,
    last_status: &mut Option<SyncStatus>,
) -> Result<(), ()> {
    emit_status(
        output,
        last_status,
        SyncStatus {
            role: SyncRole::Master,
            detail: format!("Hosting sync on TCP {}.", SYNC_PORT),
        },
    )
    .await
    .map_err(|_| ())?;

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<MasterEvent>();
    let MasterSockets { tcp, discovery } = sockets;
    spawn_discovery_responder(discovery);
    spawn_accept_loop(tcp, event_tx.clone());

    let mut next_client_id = 1u64;
    let mut clients = HashMap::<u64, UnboundedSender<WireMessage>>::new();
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                let Some(command) = command else {
                    return Err(());
                };
                match command {
                    SyncCommand::SetSnapshot(snapshot) => {
                        if is_newer(&snapshot, latest_snapshot.as_ref()) || latest_snapshot.is_none() {
                            *latest_snapshot = Some(snapshot.clone());
                        }
                        broadcast(&mut clients, &WireMessage::Snapshot(snapshot));
                    }
                    SyncCommand::RequestPoll(printer_id) => {
                        if output.send(SyncEvent::PollRequested(printer_id)).await.is_err() {
                            return Err(());
                        }
                    }
                    SyncCommand::SyncPrices(payload) => {
                        *latest_pricing_sync = Some(payload.clone());
                        broadcast(&mut clients, &WireMessage::PricingSync(payload));
                    }
                }
            }
            event = event_rx.recv() => {
                let Some(event) = event else {
                    return Err(());
                };
                match event {
                    MasterEvent::Accepted(stream, peer) => {
                        let client_id = next_client_id;
                        next_client_id = next_client_id.saturating_add(1);
                        let sender = spawn_master_client(client_id, stream, event_tx.clone());
                        clients.insert(client_id, sender.clone());
                        tracing::info!(target: SYNC_TARGET, "Client {peer} connected to sync host");
                        if let Some(snapshot) = latest_snapshot.clone() {
                            let _ = sender.send(WireMessage::Snapshot(snapshot));
                        }
                        if let Some(payload) = latest_pricing_sync.clone() {
                            let _ = sender.send(WireMessage::PricingSync(payload));
                        }
                    }
                    MasterEvent::ClientMessage(client_id, message) => {
                        match message {
                            WireMessage::Snapshot(snapshot) => {
                                if is_newer(&snapshot, latest_snapshot.as_ref()) {
                                    *latest_snapshot = Some(snapshot.clone());
                                    if output.send(SyncEvent::SnapshotReceived(snapshot.clone())).await.is_err() {
                                        return Err(());
                                    }
                                    broadcast(&mut clients, &WireMessage::Snapshot(snapshot));
                                }
                            }
                            WireMessage::PollRequest(printer_id) => {
                                if output.send(SyncEvent::PollRequested(printer_id)).await.is_err() {
                                    return Err(());
                                }
                            }
                            WireMessage::PricingSync(payload) => {
                                *latest_pricing_sync = Some(payload.clone());
                                if output.send(SyncEvent::PricingSyncReceived(payload.clone())).await.is_err() {
                                    return Err(());
                                }
                                broadcast(&mut clients, &WireMessage::PricingSync(payload));
                            }
                            WireMessage::Heartbeat => {}
                        }
                        clients.retain(|id, sender| *id != client_id || !sender.is_closed());
                    }
                    MasterEvent::ClientClosed(client_id) => {
                        clients.remove(&client_id);
                    }
                }
            }
            _ = heartbeat.tick() => {
                broadcast(&mut clients, &WireMessage::Heartbeat);
            }
        }
    }
}

async fn run_as_client(
    master_addr: SocketAddr,
    output: &mut (impl iced::futures::Sink<SyncEvent> + Unpin),
    command_rx: &mut mpsc::UnboundedReceiver<SyncCommand>,
    latest_snapshot: &mut Option<SharedState>,
    latest_pricing_sync: &mut Option<PricingSyncPayload>,
    last_status: &mut Option<SyncStatus>,
) -> Result<(), ()> {
    emit_status(
        output,
        last_status,
        SyncStatus {
            role: SyncRole::Client,
            detail: format!("Connected to sync host {}:{}.", master_addr.ip(), SYNC_PORT),
        },
    )
    .await
    .map_err(|_| ())?;

    let stream = TcpStream::connect(SocketAddr::new(master_addr.ip(), SYNC_PORT))
        .await
        .map_err(|error| {
            tracing::warn!(
                target: SYNC_TARGET,
                "Failed to connect to sync host {}:{}: {}",
                master_addr.ip(),
                SYNC_PORT,
                error
            );
        })?;

    let (reader, mut writer) = stream.into_split();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<ClientEvent>();
    spawn_client_reader(reader, event_tx);

    let mut synced_with_master = false;

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                let Some(command) = command else {
                    return Err(());
                };
                match command {
                    SyncCommand::SetSnapshot(snapshot) => {
                        *latest_snapshot = Some(snapshot.clone());
                        if synced_with_master && write_frame(&mut writer, &WireMessage::Snapshot(snapshot)).await.is_err() {
                            return Err(());
                        }
                    }
                    SyncCommand::RequestPoll(printer_id) => {
                        if write_frame(&mut writer, &WireMessage::PollRequest(printer_id)).await.is_err() {
                            return Err(());
                        }
                    }
                    SyncCommand::SyncPrices(payload) => {
                        *latest_pricing_sync = Some(payload.clone());
                        if write_frame(&mut writer, &WireMessage::PricingSync(payload)).await.is_err() {
                            return Err(());
                        }
                    }
                }
            }
            event = event_rx.recv() => {
                let Some(event) = event else {
                    return Err(());
                };
                match event {
                    ClientEvent::Message(message) => match message {
                        WireMessage::Snapshot(snapshot) => {
                            synced_with_master = true;
                            *latest_snapshot = Some(snapshot.clone());
                            if output.send(SyncEvent::SnapshotReceived(snapshot)).await.is_err() {
                                return Err(());
                            }
                        }
                        WireMessage::PricingSync(payload) => {
                            *latest_pricing_sync = Some(payload.clone());
                            if output.send(SyncEvent::PricingSyncReceived(payload)).await.is_err() {
                                return Err(());
                            }
                        }
                        WireMessage::Heartbeat => {}
                        WireMessage::PollRequest(_) => {}
                    },
                    ClientEvent::Disconnected(reason) => {
                        tracing::warn!(target: SYNC_TARGET, "Sync host disconnected: {}", reason);
                        return Err(());
                    }
                }
            }
        }
    }
}

async fn discover_master() -> io::Result<Option<SocketAddr>> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    socket.set_broadcast(true)?;

    let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), SYNC_DISCOVERY_PORT);
    let mut buffer = [0u8; 256];

    for _ in 0..DISCOVERY_ATTEMPTS {
        socket.send_to(DISCOVERY_MAGIC.as_bytes(), target).await?;

        match timeout(DISCOVERY_WAIT, socket.recv_from(&mut buffer)).await {
            Ok(Ok((len, addr))) => {
                let reply = std::str::from_utf8(&buffer[..len]).unwrap_or_default();
                if reply == DISCOVERY_RESPONSE {
                    return Ok(Some(addr));
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => {}
        }
    }

    Ok(None)
}

struct MasterSockets {
    tcp: TcpListener,
    discovery: UdpSocket,
}

impl MasterSockets {
    async fn bind() -> io::Result<Self> {
        let tcp = TcpListener::bind((Ipv4Addr::UNSPECIFIED, SYNC_PORT)).await?;
        let discovery = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, SYNC_DISCOVERY_PORT)).await?;
        discovery.set_broadcast(true)?;
        Ok(Self { tcp, discovery })
    }
}

fn spawn_accept_loop(tcp: TcpListener, event_tx: mpsc::UnboundedSender<MasterEvent>) {
    tokio::spawn(async move {
        loop {
            match tcp.accept().await {
                Ok((stream, addr)) => {
                    let _ = event_tx.send(MasterEvent::Accepted(stream, addr));
                }
                Err(error) => {
                    tracing::warn!(target: SYNC_TARGET, "Sync host accept failed: {}", error);
                    break;
                }
            }
        }
    });
}

fn spawn_discovery_responder(discovery: UdpSocket) {
    tokio::spawn(async move {
        let mut buffer = [0u8; 256];
        loop {
            match discovery.recv_from(&mut buffer).await {
                Ok((len, addr)) => {
                    if &buffer[..len] == DISCOVERY_MAGIC.as_bytes() {
                        let _ = discovery.send_to(DISCOVERY_RESPONSE.as_bytes(), addr).await;
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        target: SYNC_TARGET,
                        "Sync discovery responder failed: {}",
                        error
                    );
                    break;
                }
            }
        }
    });
}

fn spawn_master_client(
    client_id: u64,
    stream: TcpStream,
    event_tx: mpsc::UnboundedSender<MasterEvent>,
) -> UnboundedSender<WireMessage> {
    let (reader, mut writer) = stream.into_split();
    let (write_tx, mut write_rx) = mpsc::unbounded_channel::<WireMessage>();
    let read_tx = event_tx.clone();

    tokio::spawn(async move {
        let mut reader = reader;
        loop {
            match read_frame(&mut reader).await {
                Ok(message) => {
                    let _ = read_tx.send(MasterEvent::ClientMessage(client_id, message));
                }
                Err(_) => {
                    let _ = read_tx.send(MasterEvent::ClientClosed(client_id));
                    break;
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(message) = write_rx.recv().await {
            if write_frame(&mut writer, &message).await.is_err() {
                let _ = event_tx.send(MasterEvent::ClientClosed(client_id));
                break;
            }
        }
    });

    write_tx
}

fn spawn_client_reader(
    mut reader: tokio::net::tcp::OwnedReadHalf,
    event_tx: mpsc::UnboundedSender<ClientEvent>,
) {
    tokio::spawn(async move {
        loop {
            match timeout(MASTER_STALE_TIMEOUT, read_frame(&mut reader)).await {
                Ok(Ok(message)) => {
                    let _ = event_tx.send(ClientEvent::Message(message));
                }
                Ok(Err(error)) => {
                    let _ = event_tx.send(ClientEvent::Disconnected(error.to_string()));
                    break;
                }
                Err(_) => {
                    let _ = event_tx.send(ClientEvent::Disconnected(
                        "Timed out waiting for sync host heartbeat.".to_string(),
                    ));
                    break;
                }
            }
        }
    });
}

fn broadcast(clients: &mut HashMap<u64, UnboundedSender<WireMessage>>, message: &WireMessage) {
    let dead: Vec<u64> = clients
        .iter()
        .filter_map(|(client_id, sender)| sender.send(message.clone()).err().map(|_| *client_id))
        .collect();

    for client_id in dead {
        clients.remove(&client_id);
    }
}

fn is_newer(candidate: &SharedState, current: Option<&SharedState>) -> bool {
    current
        .map(|current| candidate.revision > current.revision)
        .unwrap_or(true)
}

async fn emit_status<S>(
    output: &mut S,
    last_status: &mut Option<SyncStatus>,
    status: SyncStatus,
) -> Result<(), S::Error>
where
    S: iced::futures::Sink<SyncEvent> + Unpin,
{
    if last_status.as_ref() == Some(&status) {
        return Ok(());
    }

    *last_status = Some(status.clone());
    output.send(SyncEvent::StatusChanged(status)).await
}

async fn read_frame<R>(reader: &mut R) -> io::Result<WireMessage>
where
    R: AsyncRead + Unpin,
{
    let length = reader.read_u32_le().await?;
    let mut buffer = vec![0u8; length as usize];
    reader.read_exact(&mut buffer).await?;
    let text = String::from_utf8(buffer)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    from_str(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

async fn write_frame<W>(writer: &mut W, message: &WireMessage) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let text =
        to_string(message).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let bytes = text.into_bytes();
    writer.write_u32_le(bytes.len() as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await
}
