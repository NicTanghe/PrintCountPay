use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

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
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior, sleep, timeout};

use crate::app::{
    ManualPricingBill, ManualPricingBillTombstone, ManualPricingSettings, ManualPricingWorkspace,
    PricingSettings, RecordingSession, SnmpPollStatus, StatisticsStore,
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
const MASTER_PEER_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const SYNC_TARGET: &str = "sync";
const NODE_ID_PREFIX: &str = ";node=";

static NEXT_NODE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) manual_pricing_settings: Option<ManualPricingSettings>,
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
            manual_pricing_settings: None,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatisticsSyncPayload {
    pub(crate) latest_data_at: u64,
    pub(crate) store: StatisticsStore,
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
    SyncStatistics(StatisticsSyncPayload),
}

#[derive(Debug, Clone)]
pub(crate) enum SyncEvent {
    Ready(UnboundedSender<SyncCommand>),
    StatusChanged(SyncStatus),
    SnapshotReceived(SharedState),
    PollRequested(PrinterId),
    PricingSyncReceived(PricingSyncPayload),
    StatisticsSyncReceived(StatisticsSyncPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum WireMessage {
    Snapshot(SharedState),
    PollRequest(PrinterId),
    PricingSync(PricingSyncPayload),
    StatisticsSync(StatisticsSyncPayload),
    Heartbeat,
}

#[derive(Debug)]
enum MasterEvent {
    Accepted(TcpStream, SocketAddr),
    ClientMessage(u64, WireMessage),
    ClientClosed(u64),
    PeerDiscovered(DiscoveryCandidate),
}

#[derive(Debug)]
enum ClientEvent {
    Message(WireMessage),
    Disconnected(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiscoveryCandidate {
    addr: SocketAddr,
    node_id: Option<u64>,
}

pub(crate) fn subscription() -> Subscription<SyncEvent> {
    Subscription::run(sync_worker)
}

fn sync_worker() -> impl iced::futures::Stream<Item = SyncEvent> {
    stream::channel(100, async |mut output| {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<SyncCommand>();
        let node_id = generate_node_id();
        let mut latest_snapshot = None::<SharedState>;
        let mut latest_pricing_sync = None::<PricingSyncPayload>;
        let mut latest_statistics_sync = None::<StatisticsSyncPayload>;
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

            match discover_master(Some(node_id)).await {
                Ok(Some(master)) => {
                    tracing::info!(target: SYNC_TARGET, "Discovered sync host at {}", master.addr);
                    if run_as_client(
                        master.addr,
                        &mut output,
                        &mut command_rx,
                        &mut latest_snapshot,
                        &mut latest_pricing_sync,
                        &mut latest_statistics_sync,
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
                            node_id,
                            &mut output,
                            &mut command_rx,
                            &mut latest_snapshot,
                            &mut latest_pricing_sync,
                            &mut latest_statistics_sync,
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
    node_id: u64,
    output: &mut (impl iced::futures::Sink<SyncEvent> + Unpin),
    command_rx: &mut mpsc::UnboundedReceiver<SyncCommand>,
    latest_snapshot: &mut Option<SharedState>,
    latest_pricing_sync: &mut Option<PricingSyncPayload>,
    latest_statistics_sync: &mut Option<StatisticsSyncPayload>,
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
    let _background_tasks = MasterBackgroundTasks {
        accept: spawn_accept_loop(tcp, event_tx.clone()),
        discovery: spawn_discovery_responder(discovery, node_id),
        peer_monitor: spawn_master_peer_monitor(node_id, event_tx.clone()),
    };

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
                        if let Some(snapshot) =
                            outgoing_snapshot(snapshot, latest_snapshot.as_ref())
                        {
                            *latest_snapshot = Some(snapshot.clone());
                            broadcast(&mut clients, &WireMessage::Snapshot(snapshot));
                        }
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
                    SyncCommand::SyncStatistics(payload) => {
                        if statistics_payload_is_newer(&payload, latest_statistics_sync.as_ref()) {
                            *latest_statistics_sync = Some(payload.clone());
                        }
                        broadcast(&mut clients, &WireMessage::StatisticsSync(payload));
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
                        if let Some(payload) = latest_statistics_sync.clone() {
                            let _ = sender.send(WireMessage::StatisticsSync(payload));
                        }
                    }
                    MasterEvent::ClientMessage(client_id, message) => {
                        match message {
                            WireMessage::Snapshot(snapshot) => {
                                if let Some(snapshot) =
                                    incoming_snapshot(snapshot, latest_snapshot.as_ref())
                                {
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
                            WireMessage::StatisticsSync(payload) => {
                                if statistics_payload_is_newer(&payload, latest_statistics_sync.as_ref()) {
                                    *latest_statistics_sync = Some(payload.clone());
                                }
                                if output.send(SyncEvent::StatisticsSyncReceived(payload.clone())).await.is_err() {
                                    return Err(());
                                }
                                broadcast(&mut clients, &WireMessage::StatisticsSync(payload));
                            }
                            WireMessage::Heartbeat => {}
                        }
                        clients.retain(|id, sender| *id != client_id || !sender.is_closed());
                    }
                    MasterEvent::ClientClosed(client_id) => {
                        clients.remove(&client_id);
                    }
                    MasterEvent::PeerDiscovered(candidate) => {
                        if should_yield_to_master(node_id, &candidate) {
                            tracing::info!(
                                target: SYNC_TARGET,
                                "Another sync host was found at {}; reconnecting as client",
                                candidate.addr
                            );
                            return Err(());
                        }
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
    latest_statistics_sync: &mut Option<StatisticsSyncPayload>,
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
                        let Some(snapshot) = outgoing_snapshot(snapshot, latest_snapshot.as_ref())
                        else {
                            continue;
                        };
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
                    SyncCommand::SyncStatistics(payload) => {
                        if statistics_payload_is_newer(&payload, latest_statistics_sync.as_ref()) {
                            *latest_statistics_sync = Some(payload.clone());
                        }
                        if write_frame(&mut writer, &WireMessage::StatisticsSync(payload)).await.is_err() {
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
                        WireMessage::StatisticsSync(payload) => {
                            if statistics_payload_is_newer(&payload, latest_statistics_sync.as_ref()) {
                                *latest_statistics_sync = Some(payload.clone());
                            }
                            if output.send(SyncEvent::StatisticsSyncReceived(payload)).await.is_err() {
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

async fn discover_master(local_node_id: Option<u64>) -> io::Result<Option<DiscoveryCandidate>> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    socket.set_broadcast(true)?;

    let target = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), SYNC_DISCOVERY_PORT);
    let mut buffer = [0u8; 256];

    for _ in 0..DISCOVERY_ATTEMPTS {
        socket.send_to(DISCOVERY_MAGIC.as_bytes(), target).await?;
        let wait_until = Instant::now() + DISCOVERY_WAIT;

        loop {
            let Some(remaining) = wait_until.checked_duration_since(Instant::now()) else {
                break;
            };
            if remaining.is_zero() {
                break;
            }

            match timeout(remaining, socket.recv_from(&mut buffer)).await {
                Ok(Ok((len, addr))) => {
                    let reply = std::str::from_utf8(&buffer[..len]).unwrap_or_default();
                    if let Some(node_id) = parse_discovery_response(reply) {
                        if local_node_id.is_some() && local_node_id == node_id {
                            continue;
                        }
                        return Ok(Some(DiscoveryCandidate { addr, node_id }));
                    }
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => break,
            }
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

struct MasterBackgroundTasks {
    accept: JoinHandle<()>,
    discovery: JoinHandle<()>,
    peer_monitor: JoinHandle<()>,
}

impl Drop for MasterBackgroundTasks {
    fn drop(&mut self) {
        self.accept.abort();
        self.discovery.abort();
        self.peer_monitor.abort();
    }
}

fn spawn_accept_loop(
    tcp: TcpListener,
    event_tx: mpsc::UnboundedSender<MasterEvent>,
) -> JoinHandle<()> {
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
    })
}

fn spawn_discovery_responder(discovery: UdpSocket, node_id: u64) -> JoinHandle<()> {
    tokio::spawn(async move {
        let response = discovery_response(node_id);
        let mut buffer = [0u8; 256];
        loop {
            match discovery.recv_from(&mut buffer).await {
                Ok((len, addr)) => {
                    if &buffer[..len] == DISCOVERY_MAGIC.as_bytes() {
                        let _ = discovery.send_to(response.as_bytes(), addr).await;
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
    })
}

fn spawn_master_peer_monitor(
    node_id: u64,
    event_tx: mpsc::UnboundedSender<MasterEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            sleep(MASTER_PEER_CHECK_INTERVAL).await;
            match discover_master(Some(node_id)).await {
                Ok(Some(candidate)) => {
                    if event_tx
                        .send(MasterEvent::PeerDiscovered(candidate))
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(
                        target: SYNC_TARGET,
                        "Sync host peer check failed: {}",
                        error
                    );
                }
            }
        }
    })
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

fn generate_node_id() -> u64 {
    let sequence = NEXT_NODE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let stack_marker = &sequence as *const u64 as usize;
    let mut hasher = DefaultHasher::new();
    now.hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    sequence.hash(&mut hasher);
    stack_marker.hash(&mut hasher);
    hasher.finish().max(1)
}

fn discovery_response(node_id: u64) -> String {
    format!("{DISCOVERY_RESPONSE}{NODE_ID_PREFIX}{node_id:016x}")
}

fn parse_discovery_response(reply: &str) -> Option<Option<u64>> {
    if reply == DISCOVERY_RESPONSE {
        return Some(None);
    }

    let node_id = reply
        .strip_prefix(DISCOVERY_RESPONSE)?
        .strip_prefix(NODE_ID_PREFIX)
        .and_then(|node_id| u64::from_str_radix(node_id, 16).ok())?;
    Some(Some(node_id))
}

fn should_yield_to_master(local_node_id: u64, candidate: &DiscoveryCandidate) -> bool {
    candidate
        .node_id
        .map(|remote_node_id| remote_node_id < local_node_id)
        .unwrap_or(true)
}

fn incoming_snapshot(
    mut snapshot: SharedState,
    current: Option<&SharedState>,
) -> Option<SharedState> {
    let Some(current) = current else {
        return Some(snapshot);
    };

    if snapshot.revision > current.revision {
        return Some(snapshot);
    }

    if snapshot.revision == current.revision && snapshot != *current {
        snapshot.revision = current.revision.saturating_add(1);
        return Some(snapshot);
    }

    None
}

fn outgoing_snapshot(
    mut snapshot: SharedState,
    current: Option<&SharedState>,
) -> Option<SharedState> {
    let Some(current) = current else {
        return Some(snapshot);
    };

    if snapshot.revision > current.revision {
        return Some(snapshot);
    }

    if snapshot == *current {
        return None;
    }

    snapshot.revision = current.revision.saturating_add(1);
    Some(snapshot)
}

fn statistics_payload_is_newer(
    candidate: &StatisticsSyncPayload,
    current: Option<&StatisticsSyncPayload>,
) -> bool {
    current
        .map(|current| candidate.latest_data_at > current.latest_data_at)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(revision: u64) -> SharedState {
        SharedState {
            revision,
            ..SharedState::default()
        }
    }

    #[test]
    fn discovery_response_round_trips_node_id() {
        let response = discovery_response(0x1234_abcd);

        assert_eq!(parse_discovery_response(&response), Some(Some(0x1234_abcd)));
    }

    #[test]
    fn discovery_response_accepts_legacy_master_response() {
        assert_eq!(parse_discovery_response(DISCOVERY_RESPONSE), Some(None));
    }

    #[test]
    fn master_yields_to_lower_or_legacy_peer_only() {
        let addr = SocketAddr::from(([192, 168, 1, 25], SYNC_DISCOVERY_PORT));

        assert!(should_yield_to_master(
            10,
            &DiscoveryCandidate {
                addr,
                node_id: Some(5)
            }
        ));
        assert!(!should_yield_to_master(
            10,
            &DiscoveryCandidate {
                addr,
                node_id: Some(15)
            }
        ));
        assert!(should_yield_to_master(
            10,
            &DiscoveryCandidate {
                addr,
                node_id: None
            }
        ));
    }

    #[test]
    fn incoming_snapshot_bumps_equal_revision_when_snapshot_changed() {
        let current = snapshot(5);
        let mut candidate = snapshot(5);
        candidate.pricing.bw_first_input = "0.30".to_string();

        let incoming = incoming_snapshot(candidate, Some(&current)).expect("incoming snapshot");

        assert_eq!(incoming.revision, 6);
        assert_eq!(incoming.pricing.bw_first_input, "0.30");
    }

    #[test]
    fn incoming_snapshot_rejects_equal_revision_when_snapshot_unchanged() {
        let current = snapshot(5);
        let candidate = snapshot(5);

        assert!(incoming_snapshot(candidate, Some(&current)).is_none());
    }

    #[test]
    fn incoming_snapshot_rejects_older_revision() {
        let current = snapshot(6);
        let candidate = snapshot(5);

        assert!(incoming_snapshot(candidate, Some(&current)).is_none());
    }

    #[test]
    fn outgoing_snapshot_bumps_equal_revision_local_change() {
        let current = snapshot(5);
        let mut candidate = snapshot(5);
        candidate.pricing.bw_first_input = "0.30".to_string();

        let outgoing = outgoing_snapshot(candidate, Some(&current)).expect("outgoing snapshot");

        assert_eq!(outgoing.revision, 6);
        assert_eq!(outgoing.pricing.bw_first_input, "0.30");
    }

    #[test]
    fn outgoing_snapshot_ignores_unchanged_equal_revision() {
        let current = snapshot(5);
        let candidate = snapshot(5);

        assert!(outgoing_snapshot(candidate, Some(&current)).is_none());
    }
}
