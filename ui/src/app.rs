use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iced::alignment::Horizontal;
use iced::keyboard;
use iced::theme;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Rule,
};
use iced::{Alignment, Application, Background, Border, Color, Command, Element, Length, Subscription, Theme, Vector};
use ron::de::from_str;
use ron::ser::{to_string_pretty, PrettyConfig};

use printcountpay_core::{
    default_discovery_cidr, probe_printer, resolve_counters, targets, CidrRange, CounterOidSet, Oid,
    PrinterId, PrinterRecord, PrinterStatus, SnmpAddress, SnmpConfig, SnmpRequest, SnmpResponse,
    SnmpV2cClient, SnmpVarBind, SnmpWalkRequest, DEFAULT_SNMP_PORT,
};

use crate::logging::{apply_log_level, LogEntry, LogLevel, LogStore, ReloadHandle};

const SYS_DESCR_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 1, 0];
const SYS_OBJECT_ID_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 2, 0];
const SYS_NAME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 5, 0];
const SYS_UPTIME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 3, 0];
const PRT_GENERAL_PRINTER_NAME_OID: [u32; 12] = [1, 3, 6, 1, 2, 1, 43, 5, 1, 1, 16, 1];
const PRT_MARKER_LIFECOUNT_1: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 1];
const PRT_MARKER_LIFECOUNT_2: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 2];
const PRT_MARKER_LIFECOUNT_3: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 3];
const RICOH_COUNTER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19];
const RICOH_TONER_ROOT: [u32; 12] = [1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24];
const RICOH_COLOR_COPIER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 17,
];
const RICOH_COLOR_PRINTER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 60,
];
const RICOH_BW_COPIER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 18,
];
const RICOH_BW_PRINTER_COUNT_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 61,
];
const RICOH_TONER_BLACK_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 1,
];
const RICOH_TONER_CYAN_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 2,
];
const RICOH_TONER_MAGENTA_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 3,
];
const RICOH_TONER_YELLOW_OID: [u32; 16] = [
    1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 24, 1, 1, 5, 4,
];
const PRINTER_MIB_ROOT: [u32; 7] = [1, 3, 6, 1, 2, 1, 43];
const RICOH_MIB_ROOT: [u32; 7] = [1, 3, 6, 1, 4, 1, 367];
const CRAWL_ROOTS: [&[u32]; 4] = [
    &PRINTER_MIB_ROOT,
    &RICOH_MIB_ROOT,
    &RICOH_COUNTER_ROOT,
    &RICOH_TONER_ROOT,
];
const DISCOVERY_CONCURRENCY: usize = 24;
const MAX_VARBINDS_SHOWN: usize = 200;
const FALLBACK_DISCOVERY_CIDR: &str = "192.168.129.1/24";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Printers,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterTab {
    Polling,
    Recording,
    Pricing,
    Oids,
    AddPrinters,
}

#[derive(Debug, Clone)]
pub enum Message {
    LogTick,
    LogLevelChanged(LogLevel),
    ToggleTarget(String, bool),
    CopyDiagnostics,
    AddMockSnmp,
    ManualNameChanged(String),
    ManualHostChanged(String),
    ManualPortChanged(String),
    ManualCommunityChanged(String),
    AddManualPrinter,
    PrintersPathChanged(String),
    LoadPrinters,
    SavePrinters,
    DiscoveryCidrChanged(String),
    DiscoveryCommunityChanged(String),
    StartDiscovery,
    StopDiscovery,
    DiscoveryProbeFinished(DiscoveryProbeResult),
    SelectTab(Tab),
    SelectPrinterTab(PrinterTab),
    SelectPrinter(PrinterId),
    DeleteSelectedPrinter,
    PollSelectedSnmp,
    PollExportPathChanged(String),
    ExportPollData,
    SnmpPolled {
        printer_id: PrinterId,
        result: Result<SnmpResponse, SnmpErrorInfo>,
    },
    OidsPathChanged(String),
    OidsBwChanged(String),
    OidsColorChanged(String),
    OidsTotalChanged(String),
    ApplyOids,
    LoadOids,
    SaveOids,
    CrawlOids,
    OidsCrawled(Result<CounterOidSet, SnmpErrorInfo>),
    StartRecording,
    StopRecording,
    RecordingStartChanged {
        category: RecordingCategory,
        value: String,
    },
    RecordingEndChanged {
        category: RecordingCategory,
        value: String,
    },
    RecordingToggleInclude(RecordingCategory),
    PricingBwFirstChanged(String),
    PricingBwNextChanged(String),
    PricingBwRestChanged(String),
    PricingColorChanged(String),
    PricingRoundChanged(bool),
}

#[derive(Debug, Clone)]
pub struct SnmpErrorInfo {
    summary: String,
    detail: String,
}

#[derive(Debug, Clone)]
enum SnmpPollStatus {
    Idle,
    Ok {
        received_at: u64,
        varbinds: Vec<SnmpVarBind>,
    },
    Error {
        received_at: u64,
        summary: String,
        detail: String,
    },
}

#[derive(Debug, Clone)]
struct RecordingSnapshot {
    received_at: u64,
    bw_printer: Option<u64>,
    bw_copier: Option<u64>,
    color_printer: Option<u64>,
    color_copier: Option<u64>,
    clicks_bw: Option<u64>,
    clicks_color: Option<u64>,
    clicks_total: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RecordingCategory {
    CopiesBw,
    CopiesColor,
    PrintsBw,
    PrintsColor,
}

#[derive(Debug, Clone)]
struct RecordingCategoryEdits {
    include_in_price: bool,
    start_input: String,
    end_input: String,
}

impl Default for RecordingCategoryEdits {
    fn default() -> Self {
        Self {
            include_in_price: true,
            start_input: String::new(),
            end_input: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingEdits {
    copies_bw: RecordingCategoryEdits,
    copies_color: RecordingCategoryEdits,
    prints_bw: RecordingCategoryEdits,
    prints_color: RecordingCategoryEdits,
}

impl RecordingEdits {
    fn category(&self, category: RecordingCategory) -> &RecordingCategoryEdits {
        match category {
            RecordingCategory::CopiesBw => &self.copies_bw,
            RecordingCategory::CopiesColor => &self.copies_color,
            RecordingCategory::PrintsBw => &self.prints_bw,
            RecordingCategory::PrintsColor => &self.prints_color,
        }
    }

    fn category_mut(&mut self, category: RecordingCategory) -> &mut RecordingCategoryEdits {
        match category {
            RecordingCategory::CopiesBw => &mut self.copies_bw,
            RecordingCategory::CopiesColor => &mut self.copies_color,
            RecordingCategory::PrintsBw => &mut self.prints_bw,
            RecordingCategory::PrintsColor => &mut self.prints_color,
        }
    }

    fn apply_start_snapshot(&mut self, snapshot: &RecordingSnapshot) {
        set_input(&mut self.copies_bw.start_input, snapshot.bw_copier);
        set_input(&mut self.copies_color.start_input, snapshot.color_copier);
        set_input(&mut self.prints_bw.start_input, snapshot.bw_printer);
        set_input(&mut self.prints_color.start_input, snapshot.color_printer);
        self.clear_end_inputs();
    }

    fn apply_end_snapshot(&mut self, snapshot: &RecordingSnapshot) {
        set_input(&mut self.copies_bw.end_input, snapshot.bw_copier);
        set_input(&mut self.copies_color.end_input, snapshot.color_copier);
        set_input(&mut self.prints_bw.end_input, snapshot.bw_printer);
        set_input(&mut self.prints_color.end_input, snapshot.color_printer);
    }

    fn clear_end_inputs(&mut self) {
        self.copies_bw.end_input.clear();
        self.copies_color.end_input.clear();
        self.prints_bw.end_input.clear();
        self.prints_color.end_input.clear();
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingSession {
    active: bool,
    start: Option<RecordingSnapshot>,
    end: Option<RecordingSnapshot>,
    status: Option<String>,
    edits: RecordingEdits,
}

#[derive(Debug, Clone)]
struct PricingSettings {
    bw_first_input: String,
    bw_next_input: String,
    bw_rest_input: String,
    color_input: String,
    round_to_half_euro: bool,
}

impl Default for PricingSettings {
    fn default() -> Self {
        Self {
            bw_first_input: "0.25".to_string(),
            bw_next_input: "0.10".to_string(),
            bw_rest_input: "0.06".to_string(),
            color_input: "0.50".to_string(),
            round_to_half_euro: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BwPricing {
    first_cents: u64,
    next_cents: u64,
    rest_cents: u64,
}
#[derive(Debug, Clone)]
pub struct DiscoveryProbeResult {
    run_id: u64,
    outcome: DiscoveryOutcome,
}

#[derive(Debug, Clone)]
pub enum DiscoveryOutcome {
    Printer(PrinterRecord),
    NotPrinter,
    Error(SnmpErrorInfo),
}

pub struct Flags {
    pub log_store: LogStore,
    pub reload_handle: ReloadHandle,
}

pub struct PrintCountApp {
    log_store: LogStore,
    reload_handle: ReloadHandle,
    log_entries: Vec<LogEntry>,
    log_level: LogLevel,
    known_targets: HashSet<String>,
    enabled_targets: HashSet<String>,
    copy_status: Option<String>,
    mock_snmp_count: u32,
    active_tab: Tab,
    printer_tab: PrinterTab,
    discovery_cidr: String,
    discovery_community: String,
    discovery_status: Option<String>,
    discovery_active: bool,
    discovery_queue: VecDeque<SnmpAddress>,
    discovery_in_flight: usize,
    discovery_total: usize,
    discovery_scanned: usize,
    discovery_found: usize,
    discovery_errors: usize,
    discovery_run_id: u64,
    manual_name: String,
    manual_host: String,
    manual_port: String,
    manual_community: String,
    manual_status: Option<String>,
    printers_path: String,
    printers_status: Option<String>,
    printers: Vec<PrinterRecord>,
    selected_printer: Option<PrinterId>,
    poll_states: HashMap<PrinterId, SnmpPollStatus>,
    poll_in_flight: HashSet<PrinterId>,
    poll_export_path: String,
    poll_export_status: Option<String>,
    snmp_config: SnmpConfig,
    counter_oids: CounterOidSet,
    oids_path: String,
    oids_bw_text: String,
    oids_color_text: String,
    oids_total_text: String,
    oids_status: Option<String>,
    oids_crawl_in_flight: bool,
    recording_sessions: HashMap<PrinterId, RecordingSession>,
    pricing: PricingSettings,
}

impl Application for PrintCountApp {
    type Executor = crate::executor::StackSizedTokioExecutor;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Flags) -> (Self, Command<Message>) {
        let default_targets = [
            targets::DISCOVERY,
            targets::SNMP,
            targets::POLLING,
            targets::UI,
            targets::STORAGE,
        ];
        let known_targets: HashSet<String> =
            default_targets.iter().map(|value| value.to_string()).collect();
        let enabled_targets = known_targets.clone();
        let printers = seed_printers();
        let counter_oids = default_counter_oids();
        let (oids_bw_text, oids_color_text, oids_total_text) = format_counter_oids(&counter_oids);
        let (discovery_cidr, discovery_status) = match default_discovery_cidr() {
            Some(cidr) => (cidr, None),
            None => (
                FALLBACK_DISCOVERY_CIDR.to_string(),
                Some("Local subnet not detected. Using default CIDR.".to_string()),
            ),
        };
        let mut poll_states = HashMap::new();
        for record in &printers {
            poll_states.insert(record.id.clone(), SnmpPollStatus::Idle);
        }

        (
            Self {
                log_store: flags.log_store,
                reload_handle: flags.reload_handle,
                log_entries: Vec::new(),
                log_level: LogLevel::default(),
                known_targets,
                enabled_targets,
                copy_status: None,
                mock_snmp_count: 0,
                active_tab: Tab::Printers,
                printer_tab: PrinterTab::Polling,
                discovery_cidr,
                discovery_community: "public".to_string(),
                discovery_status,
                discovery_active: false,
                discovery_queue: VecDeque::new(),
                discovery_in_flight: 0,
                discovery_total: 0,
                discovery_scanned: 0,
                discovery_found: 0,
                discovery_errors: 0,
                discovery_run_id: 0,
                manual_name: String::new(),
                manual_host: String::new(),
                manual_port: DEFAULT_SNMP_PORT.to_string(),
                manual_community: "public".to_string(),
                manual_status: None,
                printers_path: "printers.ron".to_string(),
                printers_status: None,
                printers,
                selected_printer: None,
                poll_states,
                poll_in_flight: HashSet::new(),
                poll_export_path: "polling_export.txt".to_string(),
                poll_export_status: None,
                snmp_config: SnmpConfig::default(),
                counter_oids,
                oids_path: "counter_oids.ron".to_string(),
                oids_bw_text,
                oids_color_text,
                oids_total_text,
                oids_status: None,
                oids_crawl_in_flight: false,
                recording_sessions: HashMap::new(),
                pricing: PricingSettings::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Ricoh PrintCount".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LogTick => {
                self.refresh_logs();
                Command::none()
            }
            Message::LogLevelChanged(level) => {
                self.log_level = level;
                apply_log_level(&self.reload_handle, level);
                tracing::info!(target: targets::UI, "Log level set to {}", level);
                Command::none()
            }
            Message::ToggleTarget(target, enabled) => {
                if enabled {
                    self.enabled_targets.insert(target);
                } else {
                    self.enabled_targets.remove(&target);
                }
                Command::none()
            }
            Message::CopyDiagnostics => {
                self.copy_status = Some(self.copy_diagnostics());
                Command::none()
            }
            Message::AddMockSnmp => {
                self.mock_snmp_count = self.mock_snmp_count.saturating_add(1);
                tracing::info!(
                    target: targets::SNMP,
                    count = self.mock_snmp_count,
                    "Mock SNMP entry added"
                );
                Command::none()
            }
            Message::ManualNameChanged(value) => {
                self.manual_name = value;
                Command::none()
            }
            Message::ManualHostChanged(value) => {
                self.manual_host = value;
                Command::none()
            }
            Message::ManualPortChanged(value) => {
                self.manual_port = value;
                Command::none()
            }
            Message::ManualCommunityChanged(value) => {
                self.manual_community = value;
                Command::none()
            }
            Message::AddManualPrinter => {
                self.add_manual_printer();
                Command::none()
            }
            Message::PrintersPathChanged(value) => {
                self.printers_path = value;
                Command::none()
            }
            Message::LoadPrinters => {
                self.load_printers_from_path();
                Command::none()
            }
            Message::SavePrinters => {
                self.save_printers_to_path();
                Command::none()
            }
            Message::DiscoveryCidrChanged(value) => {
                self.discovery_cidr = value;
                Command::none()
            }
            Message::DiscoveryCommunityChanged(value) => {
                self.discovery_community = value;
                Command::none()
            }
            Message::StartDiscovery => self.start_discovery(),
            Message::StopDiscovery => {
                self.stop_discovery();
                Command::none()
            }
            Message::DiscoveryProbeFinished(result) => self.handle_discovery_result(result),
            Message::SelectTab(tab) => {
                self.active_tab = tab;
                Command::none()
            }
            Message::SelectPrinterTab(tab) => {
                self.printer_tab = tab;
                Command::none()
            }
            Message::SelectPrinter(printer_id) => {
                self.selected_printer = Some(printer_id);
                self.poll_selected_printer()
            }
            Message::DeleteSelectedPrinter => {
                self.delete_selected_printer();
                Command::none()
            }
            Message::PollSelectedSnmp => self.poll_selected_printer(),
            Message::PollExportPathChanged(value) => {
                self.poll_export_path = value;
                Command::none()
            }
            Message::ExportPollData => {
                self.export_poll_data();
                Command::none()
            }
            Message::SnmpPolled { printer_id, result } => {
                self.poll_in_flight.remove(&printer_id);
                let received_at = now_epoch_seconds();
                let mut poll_name = None;
                let mut allow_override = false;
                let mut sys_descr = None;
                let state = match result {
                    Ok(response) => {
                        let printer_name = extract_text(
                            &response.varbinds,
                            &Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID),
                        );
                        let sys_name =
                            extract_text(&response.varbinds, &Oid::from_slice(&SYS_NAME_OID));
                        sys_descr =
                            extract_text(&response.varbinds, &Oid::from_slice(&SYS_DESCR_OID));
                        allow_override =
                            printer_name.is_some() || sys_name.is_some() || sys_descr.is_some();
                        poll_name = printer_name
                            .or(sys_name)
                            .or_else(|| sys_descr.clone());
                        SnmpPollStatus::Ok {
                            received_at,
                            varbinds: response.varbinds,
                        }
                    }
                    Err(error) => SnmpPollStatus::Error {
                        received_at,
                        summary: error.summary,
                        detail: error.detail,
                    },
                };
                if let Some(name) = poll_name {
                    self.apply_printer_name_fallback(
                        &printer_id,
                        name,
                        allow_override,
                        sys_descr.as_deref(),
                    );
                }
                self.poll_states.insert(printer_id, state);
                Command::none()
            }
            Message::OidsPathChanged(value) => {
                self.oids_path = value;
                Command::none()
            }
            Message::OidsBwChanged(value) => {
                self.oids_bw_text = value;
                Command::none()
            }
            Message::OidsColorChanged(value) => {
                self.oids_color_text = value;
                Command::none()
            }
            Message::OidsTotalChanged(value) => {
                self.oids_total_text = value;
                Command::none()
            }
            Message::ApplyOids => {
                self.apply_oid_inputs();
                Command::none()
            }
            Message::LoadOids => {
                self.load_oids_from_path();
                Command::none()
            }
            Message::SaveOids => {
                self.save_oids_to_path();
                Command::none()
            }
            Message::CrawlOids => self.crawl_oids(),
            Message::OidsCrawled(result) => {
                self.oids_crawl_in_flight = false;
                match result {
                    Ok(set) => {
                        let mut unique = HashSet::new();
                        unique.extend(set.bw.iter().cloned());
                        unique.extend(set.color.iter().cloned());
                        unique.extend(set.total.iter().cloned());
                        let count = unique.len();
                        self.counter_oids = set;
                        self.sync_oid_inputs();
                        self.oids_status = Some(format!(
                            "Crawl captured {count} numeric OIDs. Trim lists for faster polling."
                        ));
                    }
                    Err(error) => {
                        self.oids_status = Some(format!(
                            "Crawl failed: {} ({})",
                            error.summary, error.detail
                        ));
                    }
                }
                Command::none()
            }
            Message::StartRecording => {
                self.start_recording();
                Command::none()
            }
            Message::StopRecording => {
                self.stop_recording();
                Command::none()
            }
            Message::RecordingStartChanged { category, value } => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self
                        .recording_sessions
                        .entry(printer_id)
                        .or_default();
                    session.edits.category_mut(category).start_input = value;
                }
                Command::none()
            }
            Message::RecordingEndChanged { category, value } => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self
                        .recording_sessions
                        .entry(printer_id)
                        .or_default();
                    session.edits.category_mut(category).end_input = value;
                }
                Command::none()
            }
            Message::RecordingToggleInclude(category) => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self
                        .recording_sessions
                        .entry(printer_id)
                        .or_default();
                    let entry = session.edits.category_mut(category);
                    entry.include_in_price = !entry.include_in_price;
                }
                Command::none()
            }
            Message::PricingBwFirstChanged(value) => {
                self.pricing.bw_first_input = value;
                Command::none()
            }
            Message::PricingBwNextChanged(value) => {
                self.pricing.bw_next_input = value;
                Command::none()
            }
            Message::PricingBwRestChanged(value) => {
                self.pricing.bw_rest_input = value;
                Command::none()
            }
            Message::PricingColorChanged(value) => {
                self.pricing.color_input = value;
                Command::none()
            }
            Message::PricingRoundChanged(value) => {
                self.pricing.round_to_half_euro = value;
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let log_tick = iced::time::every(Duration::from_millis(250)).map(|_| Message::LogTick);
        let poll_tick = iced::time::every(Duration::from_secs(5)).map(|_| Message::PollSelectedSnmp);
        let delete_key = keyboard::on_key_press(delete_key_event);
        Subscription::batch(vec![log_tick, poll_tick, delete_key])
    }

    fn view(&self) -> Element<'_, Message> {
        let header = row![
            text("Ricoh PrintCount")
                .size(28)
                .style(theme::Text::Color(Color::from_rgb8(0x10, 0x1a, 0x24))),
            text("debug-first")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x5f, 0x6b, 0x7a))),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let tabs = self.tab_bar();

        let body = match self.active_tab {
            Tab::Printers => self.printers_tab_view(),
            Tab::Debug => self.debug_tab_view(),
        };

        let content = column![header, tabs, body].spacing(20).padding(16);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl PrintCountApp {
    fn refresh_logs(&mut self) {
        let entries = self.log_store.snapshot();
        for entry in &entries {
            if self.known_targets.insert(entry.target.clone()) {
                self.enabled_targets.insert(entry.target.clone());
            }
        }
        self.log_entries = entries;
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        row![
            self.tab_button(Tab::Printers, "Printers"),
            self.tab_button(Tab::Debug, "Debug")
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn tab_button(&self, tab: Tab, label: &str) -> Element<'_, Message> {
        let style = if self.active_tab == tab {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        };

        button(text(label))
            .style(style)
            .on_press(Message::SelectTab(tab))
            .into()
    }

    fn printer_tab_bar(&self) -> Element<'_, Message> {
        row![
            self.printer_tab_button(PrinterTab::Polling, "Polling"),
            self.printer_tab_button(PrinterTab::Recording, "Recording"),
            self.printer_tab_button(PrinterTab::Pricing, "Pricing"),
            self.printer_tab_button(PrinterTab::Oids, "SNMP OIDs"),
            self.printer_tab_button(PrinterTab::AddPrinters, "Discovery + Manual")
        ]
        .spacing(4)
        .align_items(Alignment::Center)
        .into()
    }

    fn printer_tab_button(&self, tab: PrinterTab, label: &str) -> Element<'_, Message> {
        let style = theme::Button::custom(FirefoxTabStyle {
            active: self.printer_tab == tab,
        });

        button(text(label))
            .padding([6, 12])
            .style(style)
            .on_press(Message::SelectPrinterTab(tab))
            .into()
    }

    fn discovery_controls_view(&self) -> Element<'_, Message> {
        let cidr_input = text_input("192.168.129.1/24", &self.discovery_cidr)
            .on_input(Message::DiscoveryCidrChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let community_input = text_input("public", &self.discovery_community)
            .on_input(Message::DiscoveryCommunityChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let action_button = if self.discovery_active {
            button("Stop").on_press(Message::StopDiscovery)
        } else {
            button("Start").on_press(Message::StartDiscovery)
        };

        let status = self
            .discovery_status
            .as_deref()
            .unwrap_or("Idle - ready to scan.");
        let progress = if self.discovery_total > 0 {
            format!(
                "Scanned {}/{} | Found {} | Errors {}",
                self.discovery_scanned,
                self.discovery_total,
                self.discovery_found,
                self.discovery_errors
            )
        } else {
            "Scanned 0/0 | Found 0 | Errors 0".to_string()
        };

        let content = column![
            text("Discovery")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("CIDR range")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                cidr_input,
            ]
            .spacing(4),
            column![
                text("Community")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                community_input,
            ]
            .spacing(4),
            row![action_button]
                .spacing(8)
                .align_items(Alignment::Center),
            text(status)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(progress)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_printer_controls_view(&self) -> Element<'_, Message> {
        let name_input = text_input("Front Office", &self.manual_name)
            .on_input(Message::ManualNameChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let host_input = text_input("192.168.1.50", &self.manual_host)
            .on_input(Message::ManualHostChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let port_input = text_input("161", &self.manual_port)
            .on_input(Message::ManualPortChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let community_input = text_input("public", &self.manual_community)
            .on_input(Message::ManualCommunityChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let status = self.manual_status.as_deref().unwrap_or("Ready.");

        let content = column![
            text("Manual add")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("Name")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                name_input,
            ]
            .spacing(4),
            column![
                text("Host or IP")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                host_input,
            ]
            .spacing(4),
            column![
                text("Port")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                port_input,
            ]
            .spacing(4),
            column![
                text("Community")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                community_input,
            ]
            .spacing(4),
            row![button("Add printer").on_press(Message::AddManualPrinter)]
                .spacing(8)
                .align_items(Alignment::Center),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_storage_controls_view(&self) -> Element<'_, Message> {
        let status = self.printers_status.as_deref().unwrap_or("Ready.");
        let path_input = text_input("printers.ron", &self.printers_path)
            .on_input(Message::PrintersPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Load").on_press(Message::LoadPrinters),
            button("Export").on_press(Message::SavePrinters),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text("Printer list storage")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("RON path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn printers_tab_view(&self) -> Element<'_, Message> {
        let list = self.printer_list_view();
        let details = self.printer_details_view();

        row![list, details]
            .spacing(16)
            .align_items(Alignment::Start)
            .into()
    }

    fn recording_tab_view(&self) -> Element<'_, Message> {
        let selected_id = self.selected_printer.as_ref();
        let selected_label = selected_id
            .and_then(|selected| {
                self.printers
                    .iter()
                    .find(|record| &record.id == selected)
                    .map(|record| {
                        record
                            .model
                            .as_deref()
                            .unwrap_or("Unknown name")
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "No printer selected".to_string());

        let selected_id_label = selected_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "None".to_string());

        let session = selected_id
            .and_then(|id| self.recording_sessions.get(id))
            .cloned()
            .unwrap_or_default();

        let status = session.status.as_deref().unwrap_or("Ready.");
        let state_label = if session.active {
            "Recording active"
        } else {
            "Recording idle"
        };

        let controls_enabled = selected_id.is_some();
        let start_button = if !controls_enabled || session.active {
            button("Start recording").style(theme::Button::Secondary)
        } else {
            button("Start recording").on_press(Message::StartRecording)
        };
        let stop_button = if !controls_enabled || !session.active {
            button("Stop recording").style(theme::Button::Secondary)
        } else {
            button("Stop recording").on_press(Message::StopRecording)
        };

        let start_time = session
            .start
            .as_ref()
            .map(|snapshot| snapshot.received_at.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        let end_time = session
            .end
            .as_ref()
            .map(|snapshot| snapshot.received_at.to_string())
            .unwrap_or_else(|| "n/a".to_string());

        let delta_section: Element<'_, Message> = if session.start.is_some() && session.end.is_some()
        {
            let copies_bw_start = category_start_value(&session, RecordingCategory::CopiesBw);
            let copies_bw_end = category_end_value(&session, RecordingCategory::CopiesBw);
            let copies_bw_delta = delta_value(copies_bw_start, copies_bw_end);

            let copies_color_start = category_start_value(&session, RecordingCategory::CopiesColor);
            let copies_color_end = category_end_value(&session, RecordingCategory::CopiesColor);
            let copies_color_delta = delta_value(copies_color_start, copies_color_end);

            let prints_bw_start = category_start_value(&session, RecordingCategory::PrintsBw);
            let prints_bw_end = category_end_value(&session, RecordingCategory::PrintsBw);
            let prints_bw_delta = delta_value(prints_bw_start, prints_bw_end);

            let prints_color_start = category_start_value(&session, RecordingCategory::PrintsColor);
            let prints_color_end = category_end_value(&session, RecordingCategory::PrintsColor);
            let prints_color_delta = delta_value(prints_color_start, prints_color_end);

            let include_copies_bw = session.edits.copies_bw.include_in_price;
            let include_copies_color = session.edits.copies_color.include_in_price;
            let include_prints_bw = session.edits.prints_bw.include_in_price;
            let include_prints_color = session.edits.prints_color.include_in_price;

            let start_bw_total = sum_two(copies_bw_start, prints_bw_start);
            let end_bw_total = sum_two(copies_bw_end, prints_bw_end);
            let total_bw_delta = delta_value(start_bw_total, end_bw_total);

            let start_color_total = sum_two(copies_color_start, prints_color_start);
            let end_color_total = sum_two(copies_color_end, prints_color_end);
            let total_color_delta = delta_value(start_color_total, end_color_total);

            let bw_delta = sum_optional_included([
                (include_copies_bw, copies_bw_delta),
                (include_prints_bw, prints_bw_delta),
            ]);
            let color_delta = sum_optional_included([
                (include_copies_color, copies_color_delta),
                (include_prints_color, prints_color_delta),
            ]);

            let bw_pricing = bw_pricing_from_settings(&self.pricing);
            let color_price = color_price_from_settings(&self.pricing);
            let bw_cost_raw = match bw_delta {
                Some(0) => Some(0),
                Some(count) => bw_pricing.map(|pricing| bw_cost_cents(count, pricing)),
                None => None,
            };
            let bw_cost_value = bw_cost_raw.map(|value| {
                if self.pricing.round_to_half_euro {
                    round_to_nearest_50_cents(value)
                } else {
                    value
                }
            });
            let color_cost_value = match color_delta {
                Some(0) => Some(0),
                Some(count) => color_price.map(|price| color_cost_cents(count, price)),
                None => None,
            };
            let subtotal_cents = match (bw_cost_value, color_cost_value) {
                (Some(bw), Some(color)) => Some(bw + color),
                _ => None,
            };
            let total_cents = subtotal_cents;
            let rounding_label = if self.pricing.round_to_half_euro {
                "B/W rounded to nearest 0.50 EUR"
            } else {
                "No rounding applied"
            };

            column![
                self.recording_table_header(),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesBw,
                    "Copies B/W",
                    &session.edits.copies_bw.start_input,
                    &session.edits.copies_bw.end_input,
                    copies_bw_delta,
                    include_copies_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesColor,
                    "Copies color",
                    &session.edits.copies_color.start_input,
                    &session.edits.copies_color.end_input,
                    copies_color_delta,
                    include_copies_color,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsBw,
                    "Prints B/W",
                    &session.edits.prints_bw.start_input,
                    &session.edits.prints_bw.end_input,
                    prints_bw_delta,
                    include_prints_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsColor,
                    "Prints color",
                    &session.edits.prints_color.start_input,
                    &session.edits.prints_color.end_input,
                    prints_color_delta,
                    include_prints_color,
                ),
                Rule::horizontal(1),
                self.recording_table_row(
                    "Total B/W",
                    start_bw_total,
                    end_bw_total,
                    total_bw_delta,
                ),
                self.recording_table_row(
                    "Total color",
                    start_color_total,
                    end_color_total,
                    total_color_delta,
                ),
                Rule::horizontal(1),
                self.value_line("Total price", total_cents.map(format_cents)),
                text(rounding_label)
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(6)
            .into()
        } else {
            text("No completed recording yet.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into()
        };

        let content = column![
            text(format!("Selected printer: {selected_label}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Recording printer ID: {selected_id_label}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(state_label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            row![start_button, stop_button]
                .spacing(8)
                .align_items(Alignment::Center),
            text(format!("Start snapshot: {start_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("End snapshot: {end_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            delta_section
        ]
        .spacing(12);

        container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn pricing_tab_view(&self) -> Element<'_, Message> {
        let bw_section = column![
            text("Black/white pricing")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.pricing_input(
                "First 5 pages (EUR)",
                "0.25",
                &self.pricing.bw_first_input,
                Message::PricingBwFirstChanged,
            ),
            self.pricing_input(
                "Next 5 pages (EUR)",
                "0.10",
                &self.pricing.bw_next_input,
                Message::PricingBwNextChanged,
            ),
            self.pricing_input(
                "Rest (EUR)",
                "0.06",
                &self.pricing.bw_rest_input,
                Message::PricingBwRestChanged,
            ),
        ]
        .spacing(6);

        let color_section = column![
            text("Color pricing")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.pricing_input(
                "Per page (EUR)",
                "0.50",
                &self.pricing.color_input,
                Message::PricingColorChanged,
            ),
        ]
        .spacing(6);

        let rounding_toggle =
            checkbox("Round B/W to nearest 0.50 EUR", self.pricing.round_to_half_euro)
                .on_toggle(Message::PricingRoundChanged)
                .size(12);

        let hint = text("Used for recording totals. Decimals accept . or ,")
            .size(11)
            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a)));

        let content = column![bw_section, color_section, rounding_toggle, hint].spacing(12);

        container(content)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_list_view(&self) -> Element<'_, Message> {
        let mut list_items = column![].spacing(6);

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered or added yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            for record in &self.printers {
                list_items = list_items.push(self.printer_row(record));
            }
        }

        let content = column![
            self.printer_storage_controls_view(),
            text("Printers")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Discovery and manual entries appear here.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            list_items,
        ]
        .spacing(12);

        let scroll = scrollable(content)
            .height(Length::Fill)
            .width(Length::Fill);

        container(scroll)
            .padding(12)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_row(&self, record: &PrinterRecord) -> Element<'_, Message> {
        let is_selected = self.selected_printer.as_ref() == Some(&record.id);
        let address = record
            .ip_or_hostname
            .as_deref()
            .or_else(|| record.snmp_address.as_ref().map(|addr| addr.host.as_str()))
            .unwrap_or("unknown host");
        let name = record.model.as_deref().unwrap_or("Unknown name");
        let status = status_label(record.status);

        let content = column![
            text(name)
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
            text(address)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text(status)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(2);

        let style = if is_selected {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        };

        button(content)
            .style(style)
            .width(Length::Fill)
            .on_press(Message::SelectPrinter(record.id.clone()))
            .into()
    }

    fn printer_details_view(&self) -> Element<'_, Message> {
        let selected_id = self.selected_printer.as_ref();
        let record = selected_id.and_then(|selected| {
            self.printers.iter().find(|record| &record.id == selected)
        });
        let selection_missing = selected_id.is_some() && record.is_none();

        let header = match self.printer_tab {
            PrinterTab::AddPrinters => column![
                text("Add printers")
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Run discovery or add a printer manually.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(4),
            _ => {
                let title = match self.printer_tab {
                    PrinterTab::Recording => "Recording",
                    PrinterTab::Pricing => "Pricing",
                    _ => "Printer details",
                };
                let mut content = column![text(title)
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)))]
                .spacing(4);

                if let Some(record) = record {
                    let address = record
                        .snmp_address
                        .as_ref()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| "Not set".to_string());
                    let name = record.model.as_deref().unwrap_or("Unknown name");
                    content = content.push(
                        text(format!("ID: {}", record.id))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                    content = content.push(
                        text(format!("Name: {}", name))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                    content = content.push(
                        text(format!("Address: {}", address))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                } else if selection_missing {
                    content = content.push(
                        text("Selected printer not found.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                }

                content
            }
        };

        let body = match self.printer_tab {
            PrinterTab::Polling => {
                if let Some(record) = record {
                    let in_flight = self.poll_in_flight.contains(&record.id);
                    let state = self
                        .poll_states
                        .get(&record.id)
                        .cloned()
                        .unwrap_or(SnmpPollStatus::Idle);
                    self.printer_poll_view(&state, in_flight)
                } else if selection_missing {
                    self.empty_printer_tab_view("Selected printer not found.")
                } else {
                    self.empty_printer_tab_view("Select a printer to start polling.")
                }
            }
            PrinterTab::Oids => {
                if let Some(record) = record {
                    self.printer_oids_view(record)
                } else if selection_missing {
                    self.empty_printer_tab_view("Selected printer not found.")
                } else {
                    self.empty_printer_tab_view("Select a printer to edit OIDs.")
                }
            }
            PrinterTab::Recording => self.recording_tab_view(),
            PrinterTab::Pricing => self.pricing_tab_view(),
            PrinterTab::AddPrinters => self.printer_add_printers_view(),
        };

        let content = column![self.printer_tab_bar(), header, body].spacing(12);

        container(content)
            .padding(12)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_add_printers_view(&self) -> Element<'_, Message> {
        column![
            self.discovery_controls_view(),
            self.manual_printer_controls_view(),
        ]
        .spacing(12)
        .into()
    }

    fn empty_printer_tab_view(&self, message: &str) -> Element<'_, Message> {
        text(message)
            .size(14)
            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
            .into()
    }

    fn printer_poll_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let content = column![
            text("Polling every 5 seconds")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            self.poll_state_view(state, in_flight),
            self.counters_view(state, in_flight),
            self.poll_export_controls_view(),
        ]
        .spacing(8);

        content.into()
    }

    fn printer_oids_view(&self, record: &PrinterRecord) -> Element<'_, Message> {
        let status = self.oids_status.as_deref().unwrap_or("No changes yet.");
        let address = record
            .snmp_address
            .as_ref()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not set".to_string());

        let path_input = text_input("counter_oids.ron", &self.oids_path)
            .on_input(Message::OidsPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Load").on_press(Message::LoadOids),
            button("Save").on_press(Message::SaveOids),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let manual_inputs = column![
            self.oids_input(
                "B/W OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.1",
                &self.oids_bw_text,
                Message::OidsBwChanged,
            ),
            self.oids_input(
                "Color OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.2",
                &self.oids_color_text,
                Message::OidsColorChanged,
            ),
            self.oids_input(
                "Total OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.3",
                &self.oids_total_text,
                Message::OidsTotalChanged,
            ),
        ]
        .spacing(8);

        let crawl_label = if self.oids_crawl_in_flight {
            "Crawling..."
        } else {
            "Crawl from printer"
        };

        let crawl_button = if self.oids_crawl_in_flight {
            button(crawl_label).style(theme::Button::Secondary)
        } else {
            button(crawl_label).on_press(Message::CrawlOids)
        };

        let actions = row![button("Apply mapping").on_press(Message::ApplyOids), crawl_button]
            .spacing(8)
            .align_items(Alignment::Center);

        let content = column![
            text("Counter OID mapping")
                .size(18)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Enter dotted OIDs separated by commas or spaces.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            column![
                text("RON path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            manual_inputs,
            actions,
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Crawl target: {address}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(
                "Crawl roots: 1.3.6.1.2.1.43, 1.3.6.1.4.1.367, 1.3.6.1.4.1.367.3.2.1.2.19, 1.3.6.1.4.1.367.3.2.1.2.24",
            )
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        content.into()
    }

    fn pricing_input(
        &self,
        label: &str,
        placeholder: &str,
        value: &str,
        on_change: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let input = text_input(placeholder, value)
            .on_input(on_change)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        column![
            text(label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            input
        ]
        .spacing(4)
        .into()
    }

    fn oids_input(
        &self,
        label: &str,
        placeholder: &str,
        value: &str,
        on_change: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let input = text_input(placeholder, value)
            .on_input(on_change)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        column![
            text(label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            input
        ]
        .spacing(4)
        .into()
    }

    fn poll_state_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let indicator = self.polling_indicator("Polling SNMP...", in_flight);
        let (last_poll, body): (String, Element<'_, Message>) = match state {
            SnmpPollStatus::Idle => (
                "Last poll: n/a".to_string(),
                text("Waiting for next poll.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                    .into(),
            ),
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let total_varbinds = varbinds.len();
                let shown_varbinds = total_varbinds.min(MAX_VARBINDS_SHOWN);
                let mut rows = column![].spacing(4);
                if varbinds.is_empty() {
                    rows = rows.push(
                        text("No varbinds returned.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                } else {
                    for varbind in varbinds.iter().take(MAX_VARBINDS_SHOWN) {
                        rows = rows.push(
                            text(format!("{} = {}", varbind.oid, varbind.value))
                                .size(13)
                                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
                        );
                    }
                    if total_varbinds > shown_varbinds {
                        rows = rows.push(
                            text(format!(
                                "Showing {shown_varbinds} of {total_varbinds} varbinds."
                            ))
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                        );
                    }
                }

                let list = scrollable(rows)
                    .height(Length::Fill)
                    .width(Length::Fill);

                let body = column![
                    text(format!("Varbinds: {shown_varbinds}/{total_varbinds}"))
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    list
                ]
                .spacing(6)
                .into();

                (format!("Last poll: {}", received_at), body)
            }
            SnmpPollStatus::Error {
                received_at,
                summary,
                detail,
            } => (
                format!("Last poll: {}", received_at),
                column![
                    text(format!("Error: {}", summary))
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
                    text(detail)
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                ]
                .spacing(4)
                .into(),
            ),
        };

        let header = row![
            text(last_poll)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a)))
                .width(Length::Fill),
            indicator,
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        column![header, body].spacing(6).into()
    }

    fn counters_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let header = row![
            text("Counters")
                .size(18)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)))
                .width(Length::Fill),
            self.polling_indicator("Polling counters...", in_flight),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let body: Element<'_, Message> = match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let resolution = resolve_counters(*received_at, &self.counter_oids, varbinds);
                let mut lines = column![
                    text("Printer counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W printer",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID),
                        ),
                    ),
                    self.value_line(
                        "Color printer",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID),
                        ),
                    ),
                    text("Copier counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W copier",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID),
                        ),
                    ),
                    self.value_line(
                        "Color copier",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID),
                        ),
                    ),
                    text("Click totals")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.counter_line("B/W clicks", resolution.snapshot.bw),
                    self.counter_line("Color clicks", resolution.snapshot.color),
                    self.counter_line("Total clicks", resolution.snapshot.total),
                    text("Toner levels")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "Black",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_BLACK_OID),
                        ),
                    ),
                    self.value_line(
                        "Cyan",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_CYAN_OID),
                        ),
                    ),
                    self.value_line(
                        "Magenta",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_MAGENTA_OID),
                        ),
                    ),
                    self.value_line(
                        "Yellow",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_YELLOW_OID),
                        ),
                    ),
                ]
                .spacing(4);

                if self.counter_oids_empty() {
                    lines = lines.push(
                        text("Counter OIDs not mapped yet.")
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    );
                }

                if !resolution.warnings.is_empty() {
                    let warning_text = resolution
                        .warnings
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<String>>()
                        .join("; ");
                    lines = lines.push(
                        text(format!("Warnings: {warning_text}"))
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    );
                }

                lines.into()
            }
            SnmpPollStatus::Idle => text("No counter data yet.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into(),
            SnmpPollStatus::Error { .. } => text("Counters unavailable due to SNMP error.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
                .into(),
        };

        let content = column![header, body].spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn polling_indicator(&self, label: &str, in_flight: bool) -> Element<'_, Message> {
        let color = if in_flight {
            Color::from_rgb8(0x3b, 0x82, 0xf6)
        } else {
            Color::TRANSPARENT
        };

        text(label)
            .size(12)
            .style(theme::Text::Color(color))
            .into()
    }

    fn poll_export_controls_view(&self) -> Element<'_, Message> {
        let status = self.poll_export_status.as_deref().unwrap_or("Ready.");
        let path_input = text_input("polling_export.txt", &self.poll_export_path)
            .on_input(Message::PollExportPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Export").on_press(Message::ExportPollData),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text("Poll export")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("File path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn counter_line(&self, label: &str, value: Option<u64>) -> Element<'_, Message> {
        let value_text = value.map(|value| value.to_string()).unwrap_or_else(|| "N/A".to_string());

        let label = text(label)
            .size(13)
            .width(Length::Fill)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let value = text(value_text)
            .size(13)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, value]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn value_line(&self, label: &str, value: Option<String>) -> Element<'_, Message> {
        let value_text = value.unwrap_or_else(|| "N/A".to_string());

        let label = text(label)
            .size(13)
            .width(Length::Fill)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let value = text(value_text)
            .size(13)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, value]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_header(&self) -> Element<'_, Message> {
        let label = text("Category")
            .size(12)
            .width(Length::FillPortion(2))
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let start = text("Start")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let end = text("End")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let delta = text("Delta")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_row(
        &self,
        label: &str,
        start: Option<u64>,
        end: Option<u64>,
        delta: Option<u64>,
    ) -> Element<'_, Message> {
        let label = text(label)
            .size(13)
            .width(Length::FillPortion(2))
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let start = text(format_count(start))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));
        let end = text(format_count(end))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));
        let delta = text(format_count(delta))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_row_editable(
        &self,
        category: RecordingCategory,
        label: &str,
        start_value: &str,
        end_value: &str,
        delta: Option<u64>,
        include_in_price: bool,
    ) -> Element<'_, Message> {
        let indicator_color = if include_in_price {
            Color::from_rgb8(0x6a, 0x6a, 0x6a)
        } else {
            Color::from_rgb8(0xe0, 0x4f, 0x4f)
        };

        let indicator = button(text("o").size(12))
            .on_press(Message::RecordingToggleInclude(category))
            .padding(2)
            .style(theme::Button::custom(IndicatorButtonStyle {
                color: indicator_color,
            }));

        let label = row![
            indicator,
            text(label)
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)))
        ]
        .spacing(6)
        .align_items(Alignment::Center)
        .width(Length::FillPortion(2));

        let start = text_input("n/a", start_value)
            .on_input(move |value| Message::RecordingStartChanged { category, value })
            .padding(4)
            .size(12)
            .width(Length::FillPortion(1));
        let end = text_input("n/a", end_value)
            .on_input(move |value| Message::RecordingEndChanged { category, value })
            .padding(4)
            .size(12)
            .width(Length::FillPortion(1));
        let delta = text(format_count(delta))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn debug_tab_view(&self) -> Element<'_, Message> {
        let level_picker = pick_list(
            &LogLevel::ALL[..],
            Some(self.log_level),
            Message::LogLevelChanged,
        )
        .placeholder("Log level");

        let console_header = row![
            text("Console")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            level_picker
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let log_lines = self.log_lines_view();
        let filters = self.target_filters_view();

        let console = column![console_header, filters, log_lines]
            .spacing(12)
            .width(Length::FillPortion(2));

        let debug_panel = self.debug_panel_view();

        row![console, debug_panel]
            .spacing(16)
            .align_items(Alignment::Start)
            .into()
    }

    fn target_filters_view(&self) -> Element<'_, Message> {
        let mut filter_column = column![
            text("Targets")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)))
        ]
        .spacing(6);

        for target in self.sorted_targets() {
            let enabled = self.enabled_targets.contains(&target);
            filter_column = filter_column.push(
                checkbox(target.clone(), enabled)
                    .on_toggle(move |value| Message::ToggleTarget(target.clone(), value)),
            );
        }

        container(filter_column)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn log_lines_view(&self) -> Element<'_, Message> {
        let mut lines = column![].spacing(4);

        for entry in self.visible_entries() {
            let color = level_color(entry.level);
            let line = text(entry.format_line())
                .size(14)
                .horizontal_alignment(Horizontal::Left)
                .style(theme::Text::Color(color));
            lines = lines.push(line);
        }

        scrollable(lines)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn debug_panel_view(&self) -> Element<'_, Message> {
        let copy_status = self.copy_status.as_deref().unwrap_or("Ready");
        let panel = column![
            text("Debug panel")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Per-printer errors: none recorded yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text("SNMP OIDs used: not captured yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text("Persistence diagnostics: not captured yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text(format!("Mock SNMP entries: {}", self.mock_snmp_count))
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            button("Add mock SNMP entry").on_press(Message::AddMockSnmp),
            button("Copy diagnostics").on_press(Message::CopyDiagnostics),
            text(format!("Clipboard: {copy_status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(10);

        container(panel)
            .padding(12)
            .width(Length::FillPortion(1))
            .style(theme::Container::Box)
            .into()
    }

    fn sorted_targets(&self) -> Vec<String> {
        let mut targets: Vec<String> = self.known_targets.iter().cloned().collect();
        targets.sort();
        targets
    }

    fn visible_entries(&self) -> Vec<&LogEntry> {
        self.log_entries
            .iter()
            .filter(|entry| self.enabled_targets.contains(&entry.target))
            .collect()
    }

    fn copy_diagnostics(&self) -> String {
        let text = self.diagnostics_text();
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
            Ok(()) => {
                tracing::info!(target: targets::UI, "Diagnostics copied to clipboard");
                "Copied".to_string()
            }
            Err(error) => {
                tracing::warn!(target: targets::UI, "Clipboard copy failed: {}", error);
                format!("Failed: {error}")
            }
        }
    }

    fn diagnostics_text(&self) -> String {
        let mut output = String::new();
        output.push_str("PrintCount diagnostics\n");
        output.push_str(&format!("Log level: {}\n", self.log_level));
        if let Some(selected) = &self.selected_printer {
            output.push_str(&format!("Selected printer: {}\n", selected));
        }
        output.push_str(&format!("Mock SNMP entries: {}\n", self.mock_snmp_count));
        output.push_str(&format!(
            "Targets enabled: {}\n",
            self.sorted_targets()
                .into_iter()
                .filter(|target| self.enabled_targets.contains(target))
                .collect::<Vec<String>>()
                .join(", ")
        ));
        output.push_str("Per-printer errors: none recorded yet\n");
        output.push_str("SNMP OIDs used: not captured yet\n");
        output.push_str("Persistence diagnostics: not captured yet\n");
        output.push_str("Recent logs:\n");

        let entries = self.visible_entries();
        let start = entries.len().saturating_sub(50);
        for entry in entries.into_iter().skip(start) {
            output.push_str(&entry.format_line());
            output.push('\n');
        }

        output
    }

    fn start_discovery(&mut self) -> Command<Message> {
        let cidr = self.discovery_cidr.trim();
        if cidr.is_empty() {
            self.discovery_status = Some("CIDR is empty.".to_string());
            return Command::none();
        }

        let range = match CidrRange::parse(cidr) {
            Ok(range) => range,
            Err(error) => {
                self.discovery_status = Some(format!("Invalid CIDR: {error}"));
                return Command::none();
            }
        };

        let mut queue = VecDeque::new();
        for ip in range.iter() {
            queue.push_back(SnmpAddress::with_default_port(ip.to_string()));
        }

        if queue.is_empty() {
            self.discovery_status = Some("CIDR contains no usable hosts.".to_string());
            return Command::none();
        }

        self.discovery_run_id = self.discovery_run_id.wrapping_add(1);
        self.discovery_active = true;
        self.discovery_queue = queue;
        self.discovery_total = self.discovery_queue.len();
        self.discovery_scanned = 0;
        self.discovery_found = 0;
        self.discovery_errors = 0;
        self.discovery_in_flight = 0;
        self.discovery_status = Some(format!(
            "Discovery started ({} hosts).",
            self.discovery_total
        ));

        self.spawn_discovery_tasks()
    }

    fn stop_discovery(&mut self) {
        self.discovery_active = false;
        self.discovery_queue.clear();
        self.discovery_in_flight = 0;
        self.discovery_run_id = self.discovery_run_id.wrapping_add(1);
        self.discovery_status = Some("Discovery stopped.".to_string());
    }

    fn handle_discovery_result(&mut self, result: DiscoveryProbeResult) -> Command<Message> {
        if result.run_id != self.discovery_run_id {
            return Command::none();
        }

        self.discovery_in_flight = self.discovery_in_flight.saturating_sub(1);
        self.discovery_scanned = self.discovery_scanned.saturating_add(1);

        match result.outcome {
            DiscoveryOutcome::Printer(record) => {
                self.discovery_found = self.discovery_found.saturating_add(1);
                self.upsert_printer(record);
            }
            DiscoveryOutcome::NotPrinter => {}
            DiscoveryOutcome::Error(error) => {
                self.discovery_errors = self.discovery_errors.saturating_add(1);
                self.discovery_status = Some(format!(
                    "Last error: {} ({})",
                    error.summary, error.detail
                ));
            }
        }

        if self.discovery_queue.is_empty() && self.discovery_in_flight == 0 {
            self.discovery_active = false;
            self.discovery_status = Some(format!(
                "Discovery complete: {} printers found.",
                self.discovery_found
            ));
            return Command::none();
        }

        self.spawn_discovery_tasks()
    }

    fn spawn_discovery_tasks(&mut self) -> Command<Message> {
        if !self.discovery_active {
            return Command::none();
        }

        let mut commands = Vec::new();
        while self.discovery_in_flight < DISCOVERY_CONCURRENCY {
            let Some(address) = self.discovery_queue.pop_front() else {
                break;
            };

            let run_id = self.discovery_run_id;
            let community = self.discovery_community.trim().to_string();
            let community = (!community.is_empty()).then_some(community);
            let config = self.snmp_config.clone();

            self.discovery_in_flight += 1;
            commands.push(Command::perform(
                async move {
                    let result = probe_printer(address, community, config).await;
                    let outcome = match result {
                        Ok(Some(record)) => DiscoveryOutcome::Printer(record),
                        Ok(None) => DiscoveryOutcome::NotPrinter,
                        Err(error) => DiscoveryOutcome::Error(SnmpErrorInfo {
                            summary: error.user_summary(),
                            detail: error.technical_detail(),
                        }),
                    };
                    DiscoveryProbeResult { run_id, outcome }
                },
                Message::DiscoveryProbeFinished,
            ));
        }

        Command::batch(commands)
    }

    fn upsert_printer(&mut self, record: PrinterRecord) {
        let host = record
            .snmp_address
            .as_ref()
            .map(|addr| addr.host.as_str());

        if let Some(existing) = self.printers.iter_mut().find(|printer| {
            printer
                .snmp_address
                .as_ref()
                .map(|addr| addr.host.as_str())
                == host
        }) {
            existing.ip_or_hostname = record.ip_or_hostname;
            existing.model = record.model;
            existing.sys_object_id = record.sys_object_id;
            existing.snmp_address = record.snmp_address;
            existing.community = record.community;
            existing.status = record.status;
            existing.last_seen = record.last_seen;
        } else {
            self.poll_states
                .insert(record.id.clone(), SnmpPollStatus::Idle);
            self.printers.push(record);
        }
    }

    fn delete_selected_printer(&mut self) {
        if self.active_tab != Tab::Printers {
            return;
        }

        let Some(selected) = self.selected_printer.clone() else {
            return;
        };

        let Some(index) = self.printers.iter().position(|record| record.id == selected) else {
            self.selected_printer = None;
            return;
        };

        self.printers.remove(index);
        self.poll_states.remove(&selected);
        self.poll_in_flight.remove(&selected);
        self.recording_sessions.remove(&selected);

        if self.printers.is_empty() {
            self.selected_printer = None;
            return;
        }

        let new_index = index.min(self.printers.len() - 1);
        self.selected_printer = Some(self.printers[new_index].id.clone());
    }

    fn find_printer_by_host_mut(&mut self, host: &str) -> Option<&mut PrinterRecord> {
        self.printers.iter_mut().find(|printer| {
            printer
                .snmp_address
                .as_ref()
                .map(|addr| addr.host.as_str())
                == Some(host)
                || printer.ip_or_hostname.as_deref() == Some(host)
        })
    }

    fn add_manual_printer(&mut self) {
        let name = self.manual_name.trim().to_string();
        let host = self.manual_host.trim().to_string();
        let port_text = self.manual_port.trim().to_string();
        let community = self.manual_community.trim().to_string();

        if host.is_empty() {
            self.manual_status = Some("Add failed: host is empty.".to_string());
            return;
        }

        let port = if port_text.is_empty() {
            DEFAULT_SNMP_PORT
        } else {
            match port_text.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    self.manual_status = Some("Add failed: invalid port.".to_string());
                    return;
                }
            }
        };

        let now = now_epoch_seconds();
        if let Some(existing) = self.find_printer_by_host_mut(&host) {
            if !name.is_empty() {
                existing.model = Some(name);
            }
            existing.ip_or_hostname = Some(host.clone());
            existing.snmp_address = Some(SnmpAddress::new(host.clone(), port));
            if !community.is_empty() {
                existing.community = Some(community);
            }
            existing.last_seen = Some(now);
            self.manual_status = Some(format!("Updated printer {host}."));
            return;
        }

        let mut record = PrinterRecord::new(PrinterId::new(format!("manual-{host}")));
        record.ip_or_hostname = Some(host.clone());
        record.model = (!name.is_empty()).then_some(name);
        record.snmp_address = Some(SnmpAddress::new(host.clone(), port));
        record.community = (!community.is_empty()).then_some(community);
        record.last_seen = Some(now);

        self.poll_states
            .insert(record.id.clone(), SnmpPollStatus::Idle);
        self.printers.push(record);
        self.manual_name.clear();
        self.manual_host.clear();
        self.manual_status = Some(format!("Added printer {host}."));
    }

    fn apply_printer_name_fallback(
        &mut self,
        printer_id: &PrinterId,
        name: String,
        allow_override: bool,
        sys_descr: Option<&str>,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let Some(record) = self
            .printers
            .iter_mut()
            .find(|record| &record.id == printer_id)
        else {
            return;
        };

        let existing = record
            .model
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        let is_manual = record.id.0.starts_with("manual-");

        if existing.is_empty() {
            record.model = Some(name.to_string());
            return;
        }

        if is_manual {
            return;
        }

        if !allow_override {
            return;
        }

        let mut should_replace = false;
        if let Some(sys_descr) = sys_descr.map(str::trim) {
            if !sys_descr.is_empty() && existing == sys_descr {
                should_replace = true;
            }
        }
        if let Some(host) = record.ip_or_hostname.as_deref().map(str::trim) {
            if !host.is_empty() && existing == host {
                should_replace = true;
            }
        }

        if should_replace && existing != name {
            record.model = Some(name.to_string());
        }
    }

    fn load_printers_from_path(&mut self) {
        let path = self.printers_path.trim().to_string();
        if path.is_empty() {
            self.printers_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<Vec<PrinterRecord>>(&contents) {
                Ok(printers) => {
                    let count = printers.len();
                    self.replace_printers(printers);
                    self.printers_status = Some(format!("Loaded {count} printers from {path}."));
                }
                Err(error) => {
                    self.printers_status = Some(format!("Load failed: {error}"));
                }
            },
            Err(error) => {
                self.printers_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn save_printers_to_path(&mut self) {
        let path = self.printers_path.trim().to_string();
        if path.is_empty() {
            self.printers_status = Some("Save failed: path is empty.".to_string());
            return;
        }

        let config = PrettyConfig::new();
        match to_string_pretty(&self.printers, config) {
            Ok(contents) => match fs::write(&path, contents) {
                Ok(()) => {
                    self.printers_status = Some(format!(
                        "Saved {} printers to {path}.",
                        self.printers.len()
                    ));
                }
                Err(error) => {
                    self.printers_status = Some(format!("Save failed: {error}"));
                }
            },
            Err(error) => {
                self.printers_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn replace_printers(&mut self, printers: Vec<PrinterRecord>) {
        let selected = self.selected_printer.clone();
        self.printers = printers;
        self.poll_states.clear();
        self.poll_in_flight.clear();
        self.recording_sessions
            .retain(|id, _| self.printers.iter().any(|record| &record.id == id));

        for record in &self.printers {
            self.poll_states
                .insert(record.id.clone(), SnmpPollStatus::Idle);
        }

        if let Some(selected) = selected {
            if self.printers.iter().any(|record| record.id == selected) {
                self.selected_printer = Some(selected);
            } else {
                self.selected_printer = None;
            }
        }
    }

    fn poll_selected_printer(&mut self) -> Command<Message> {
        let Some(printer_id) = self.selected_printer.clone() else {
            return Command::none();
        };

        if self.poll_in_flight.contains(&printer_id) {
            return Command::none();
        }

        let Some(record) = self.printers.iter().find(|record| record.id == printer_id) else {
            return Command::none();
        };

        let now = now_epoch_seconds();
        let Some(address) = record.snmp_address.clone() else {
            self.poll_states.insert(
                printer_id,
                SnmpPollStatus::Error {
                    received_at: now,
                    summary: "Missing SNMP address".to_string(),
                    detail: "Printer has no SNMP address configured.".to_string(),
                },
            );
            return Command::none();
        };

        let mut request = SnmpRequest::new(address, snmp_oids(&self.counter_oids));
        if let Some(community) = record.community.clone() {
            request = request.with_community(community);
        }

        let config = self.snmp_config.clone();
        let printer_id = printer_id.clone();

        self.poll_in_flight.insert(printer_id.clone());
        self.poll_states
            .entry(printer_id.clone())
            .or_insert(SnmpPollStatus::Idle);

        Command::perform(
            async move {
                let client = SnmpV2cClient::new(config);
                match client.get(request).await {
                    Ok(response) => Ok(response),
                    Err(error) => Err(SnmpErrorInfo {
                        summary: error.user_summary(),
                        detail: error.technical_detail(),
                    }),
                }
            },
            move |result| Message::SnmpPolled { printer_id, result },
        )
    }

    fn start_recording(&mut self) {
        let Some(printer_id) = self.selected_printer.clone() else {
            return;
        };

        let already_active = self
            .recording_sessions
            .get(&printer_id)
            .map(|session| session.active)
            .unwrap_or(false);
        if already_active {
            let session = self
                .recording_sessions
                .entry(printer_id.clone())
                .or_default();
            session.status = Some("Start ignored: recording already active.".to_string());
            return;
        }

        let snapshot_result = self.snapshot_for_printer(&printer_id);
        let session = self
            .recording_sessions
            .entry(printer_id.clone())
            .or_default();

        match snapshot_result {
            Ok(snapshot) => {
                session.active = true;
                session.start = Some(snapshot.clone());
                session.end = None;
                session.edits.apply_start_snapshot(&snapshot);
                session.status = Some(format!(
                    "Recording started at {}.",
                    snapshot.received_at
                ));
            }
            Err(error) => {
                session.status = Some(format!("Start failed: {error}"));
            }
        }
    }

    fn stop_recording(&mut self) {
        let Some(printer_id) = self.selected_printer.clone() else {
            return;
        };

        let is_active = self
            .recording_sessions
            .get(&printer_id)
            .map(|session| session.active)
            .unwrap_or(false);
        if !is_active {
            let session = self
                .recording_sessions
                .entry(printer_id.clone())
                .or_default();
            session.status = Some("Stop failed: no active recording.".to_string());
            return;
        }

        let snapshot_result = self.snapshot_for_printer(&printer_id);
        let session = self
            .recording_sessions
            .entry(printer_id.clone())
            .or_default();

        match snapshot_result {
            Ok(snapshot) => {
                session.active = false;
                session.end = Some(snapshot.clone());
                session.edits.apply_end_snapshot(&snapshot);
                session.status = Some(format!(
                    "Recording stopped at {}.",
                    snapshot.received_at
                ));
            }
            Err(error) => {
                session.status = Some(format!("Stop failed: {error}"));
            }
        }
    }

    fn export_poll_data(&mut self) {
        let path = self.poll_export_path.trim().to_string();
        if path.is_empty() {
            self.poll_export_status = Some("Export failed: path is empty.".to_string());
            return;
        }

        let Some(printer_id) = self.selected_printer.clone() else {
            self.poll_export_status = Some("Export failed: select a printer first.".to_string());
            return;
        };

        let Some(state) = self.poll_states.get(&printer_id) else {
            self.poll_export_status = Some("Export failed: no poll data yet.".to_string());
            return;
        };

        let SnmpPollStatus::Ok {
            received_at,
            varbinds,
        } = state
        else {
            self.poll_export_status = Some("Export failed: no poll data yet.".to_string());
            return;
        };

        let (name, address) = match self
            .printers
            .iter()
            .find(|record| record.id == printer_id)
        {
            Some(record) => {
                let name = record.model.as_deref().unwrap_or("Unknown name").to_string();
                let address = record
                    .snmp_address
                    .as_ref()
                    .map(|addr| addr.to_string())
                    .or_else(|| record.ip_or_hostname.clone())
                    .unwrap_or_else(|| "Not set".to_string());
                (name, address)
            }
            None => ("Unknown name".to_string(), "Not set".to_string()),
        };

        let mut contents = String::new();
        let mut push_line = |line: &str| {
            contents.push_str(line);
            contents.push('\n');
        };

        push_line("PrintCountPay poll export");
        push_line(&format!("printer_id={printer_id}"));
        push_line(&format!("name={name}"));
        push_line(&format!("address={address}"));
        push_line(&format!("received_at={received_at}"));
        push_line("");

        if varbinds.is_empty() {
            push_line("No varbinds returned.");
        } else {
            for varbind in varbinds {
                push_line(&format!("{} = {}", varbind.oid, varbind.value));
            }
        }

        match fs::write(&path, contents) {
            Ok(()) => {
                self.poll_export_status = Some(format!("Exported poll data to {path}."));
            }
            Err(error) => {
                self.poll_export_status = Some(format!("Export failed: {error}"));
            }
        }
    }

    fn snapshot_for_printer(
        &self,
        printer_id: &PrinterId,
    ) -> Result<RecordingSnapshot, String> {
        let Some(state) = self.poll_states.get(printer_id) else {
            return Err("No poll data yet.".to_string());
        };

        match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => Ok(self.build_recording_snapshot(*received_at, varbinds)),
            SnmpPollStatus::Error { summary, detail, .. } => {
                Err(format!("{summary} ({detail})"))
            }
            SnmpPollStatus::Idle => Err("No poll data yet.".to_string()),
        }
    }

    fn build_recording_snapshot(
        &self,
        received_at: u64,
        varbinds: &[SnmpVarBind],
    ) -> RecordingSnapshot {
        let resolution = resolve_counters(received_at, &self.counter_oids, varbinds);
        RecordingSnapshot {
            received_at,
            bw_printer: extract_counter_u64(varbinds, &Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID)),
            bw_copier: extract_counter_u64(varbinds, &Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID)),
            color_printer: extract_counter_u64(
                varbinds,
                &Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID),
            ),
            color_copier: extract_counter_u64(
                varbinds,
                &Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID),
            ),
            clicks_bw: resolution.snapshot.bw,
            clicks_color: resolution.snapshot.color,
            clicks_total: resolution.snapshot.total,
        }
    }

    fn sync_oid_inputs(&mut self) {
        let (bw, color, total) = format_counter_oids(&self.counter_oids);
        self.oids_bw_text = bw;
        self.oids_color_text = color;
        self.oids_total_text = total;
    }

    fn apply_oid_inputs(&mut self) {
        match self.parse_oid_inputs() {
            Ok(set) => {
                self.counter_oids = set;
                self.oids_status = Some("Applied OID mapping.".to_string());
            }
            Err(error) => {
                self.oids_status = Some(format!("Apply failed: {error}"));
            }
        }
    }

    fn parse_oid_inputs(&self) -> Result<CounterOidSet, String> {
        let bw = parse_oid_list(&self.oids_bw_text)
            .map_err(|error| format!("B/W OIDs: {error}"))?;
        let color = parse_oid_list(&self.oids_color_text)
            .map_err(|error| format!("Color OIDs: {error}"))?;
        let total = parse_oid_list(&self.oids_total_text)
            .map_err(|error| format!("Total OIDs: {error}"))?;

        Ok(CounterOidSet { bw, color, total })
    }

    fn load_oids_from_path(&mut self) {
        let path = self.oids_path.trim().to_string();
        if path.is_empty() {
            self.oids_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<CounterOidSet>(&contents) {
                Ok(set) => {
                    self.counter_oids = set;
                    self.sync_oid_inputs();
                    self.oids_status = Some(format!("Loaded OIDs from {path}."));
                }
                Err(error) => {
                    self.oids_status = Some(format!("Load failed: {error}"));
                }
            },
            Err(error) => {
                self.oids_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn save_oids_to_path(&mut self) {
        let path = self.oids_path.trim().to_string();
        if path.is_empty() {
            self.oids_status = Some("Save failed: path is empty.".to_string());
            return;
        }

        let config = PrettyConfig::new();
        match to_string_pretty(&self.counter_oids, config) {
            Ok(contents) => match fs::write(&path, contents) {
                Ok(()) => {
                    self.oids_status = Some(format!("Saved OIDs to {path}."));
                }
                Err(error) => {
                    self.oids_status = Some(format!("Save failed: {error}"));
                }
            },
            Err(error) => {
                self.oids_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn crawl_oids(&mut self) -> Command<Message> {
        if self.oids_crawl_in_flight {
            return Command::none();
        }

        let Some(printer_id) = self.selected_printer.clone() else {
            self.oids_status = Some("Crawl failed: select a printer first.".to_string());
            return Command::none();
        };

        let Some(record) = self.printers.iter().find(|record| record.id == printer_id) else {
            self.oids_status = Some("Crawl failed: selected printer missing.".to_string());
            return Command::none();
        };

        let Some(address) = record.snmp_address.clone() else {
            self.oids_status = Some("Crawl failed: printer has no SNMP address.".to_string());
            return Command::none();
        };

        let community = record.community.clone();
        let config = self.snmp_config.clone();
        self.oids_crawl_in_flight = true;
        self.oids_status = Some("Crawling printer/vendor MIBs...".to_string());

        Command::perform(
            async move {
                let client = SnmpV2cClient::new(config);
                let mut varbinds = Vec::new();
                let mut last_error = None;

                for root in CRAWL_ROOTS {
                    let mut request =
                        SnmpWalkRequest::new(address.clone(), Oid::from_slice(root))
                            .with_max_results(0);
                    if let Some(ref community) = community {
                        request = request.with_community(community.clone());
                    }

                    match client.walk(request).await {
                        Ok(response) => varbinds.extend(response.varbinds),
                        Err(error) => {
                            last_error = Some(SnmpErrorInfo {
                                summary: error.user_summary(),
                                detail: error.technical_detail(),
                            });
                        }
                    }
                }

                if varbinds.is_empty() {
                    Err(last_error.unwrap_or(SnmpErrorInfo {
                        summary: "Crawl failed.".to_string(),
                        detail: "No OIDs returned from crawl.".to_string(),
                    }))
                } else {
                    Ok(counter_oids_from_walk(&varbinds))
                }
            },
            Message::OidsCrawled,
        )
    }

    fn counter_oids_empty(&self) -> bool {
        self.counter_oids.bw.is_empty()
            && self.counter_oids.color.is_empty()
            && self.counter_oids.total.is_empty()
    }
}

fn level_color(level: tracing::Level) -> Color {
    match level {
        tracing::Level::ERROR => Color::from_rgb8(0xe0, 0x4f, 0x4f),
        tracing::Level::WARN => Color::from_rgb8(0xe0, 0xb0, 0x4f),
        tracing::Level::INFO => Color::from_rgb8(0x3b, 0x82, 0xf6),
        tracing::Level::DEBUG => Color::from_rgb8(0x22, 0x7d, 0x64),
        tracing::Level::TRACE => Color::from_rgb8(0x6b, 0x72, 0x80),
    }
}

fn delete_key_event(
    key: keyboard::Key,
    _modifiers: keyboard::Modifiers,
) -> Option<Message> {
    match key {
        keyboard::Key::Named(keyboard::key::Named::Delete) => {
            Some(Message::DeleteSelectedPrinter)
        }
        _ => None,
    }
}

fn status_label(status: PrinterStatus) -> &'static str {
    match status {
        PrinterStatus::Unknown => "Unknown",
        PrinterStatus::Online => "Online",
        PrinterStatus::Offline => "Offline",
        PrinterStatus::Error => "Error",
    }
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn default_counter_oids() -> CounterOidSet {
    CounterOidSet {
        bw: vec![
            Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID),
            Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID),
            Oid::from_slice(&PRT_MARKER_LIFECOUNT_1),
        ],
        color: vec![
            Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID),
            Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID),
            Oid::from_slice(&PRT_MARKER_LIFECOUNT_2),
        ],
        total: vec![Oid::from_slice(&PRT_MARKER_LIFECOUNT_3)],
    }
}

fn format_oid_list(oids: &[Oid]) -> String {
    oids.iter()
        .map(|oid| oid.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

fn format_counter_oids(oids: &CounterOidSet) -> (String, String, String) {
    (
        format_oid_list(&oids.bw),
        format_oid_list(&oids.color),
        format_oid_list(&oids.total),
    )
}

fn parse_oid_list(value: &str) -> Result<Vec<Oid>, String> {
    let mut oids = Vec::new();
    for token in value.split(|ch: char| ch == ',' || ch.is_whitespace()) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let oid = token
            .parse::<Oid>()
            .map_err(|error| format!("invalid OID '{token}': {error}"))?;
        oids.push(oid);
    }
    Ok(oids)
}

fn extract_text(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<String> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    let value = varbind
        .value
        .as_text_lossy()
        .unwrap_or_else(|| varbind.value.to_string());
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_value_string(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<String> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    if let Some(value) = varbind.value.as_u64() {
        return Some(value.to_string());
    }
    Some(varbind.value.to_string())
}

fn extract_counter_u64(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<u64> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    varbind.value.as_u64()
}

fn delta_value(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    let start = start?;
    let end = end?;
    end.checked_sub(start)
}

fn sum_two(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    Some(left? + right?)
}

fn bw_cost_cents(count: u64, pricing: BwPricing) -> u64 {
    let first = count.min(5);
    let second = count.saturating_sub(5).min(5);
    let rest = count.saturating_sub(10);
    first * pricing.first_cents + second * pricing.next_cents + rest * pricing.rest_cents
}

fn color_cost_cents(count: u64, price_cents: u64) -> u64 {
    count * price_cents
}

fn round_to_nearest_50_cents(total_cents: u64) -> u64 {
    (total_cents + 25) / 50 * 50
}

fn format_cents(cents: u64) -> String {
    let euros = cents / 100;
    let remainder = cents % 100;
    format!("{euros}.{remainder:02} EUR")
}

fn format_count(value: Option<u64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| "N/A".to_string())
}

fn set_input(target: &mut String, value: Option<u64>) {
    target.clear();
    if let Some(value) = value {
        target.push_str(&value.to_string());
    }
}

fn parse_count_input(value: &str) -> Result<Option<u64>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed.parse::<u64>().map(Some).map_err(|_| ())
}

fn parse_price_input(value: &str) -> Result<Option<u64>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.replace(',', ".");
    let parsed = normalized.parse::<f64>().map_err(|_| ())?;
    if parsed < 0.0 {
        return Err(());
    }
    Ok(Some((parsed * 100.0).round() as u64))
}

fn bw_pricing_from_settings(settings: &PricingSettings) -> Option<BwPricing> {
    let first = parse_price_input(&settings.bw_first_input).ok().flatten()?;
    let next = parse_price_input(&settings.bw_next_input).ok().flatten()?;
    let rest = parse_price_input(&settings.bw_rest_input).ok().flatten()?;
    Some(BwPricing {
        first_cents: first,
        next_cents: next,
        rest_cents: rest,
    })
}

fn color_price_from_settings(settings: &PricingSettings) -> Option<u64> {
    parse_price_input(&settings.color_input).ok().flatten()
}

fn snapshot_category_value(
    snapshot: &RecordingSnapshot,
    category: RecordingCategory,
) -> Option<u64> {
    match category {
        RecordingCategory::CopiesBw => snapshot.bw_copier,
        RecordingCategory::CopiesColor => snapshot.color_copier,
        RecordingCategory::PrintsBw => snapshot.bw_printer,
        RecordingCategory::PrintsColor => snapshot.color_printer,
    }
}

fn category_start_value(
    session: &RecordingSession,
    category: RecordingCategory,
) -> Option<u64> {
    let edits = session.edits.category(category);
    match parse_count_input(&edits.start_input) {
        Ok(Some(value)) => Some(value),
        Ok(None) => session
            .start
            .as_ref()
            .and_then(|snapshot| snapshot_category_value(snapshot, category)),
        Err(()) => None,
    }
}

fn category_end_value(
    session: &RecordingSession,
    category: RecordingCategory,
) -> Option<u64> {
    let edits = session.edits.category(category);
    match parse_count_input(&edits.end_input) {
        Ok(Some(value)) => Some(value),
        Ok(None) => session
            .end
            .as_ref()
            .and_then(|snapshot| snapshot_category_value(snapshot, category)),
        Err(()) => None,
    }
}

fn sum_optional_included(
    values: impl IntoIterator<Item = (bool, Option<u64>)>,
) -> Option<u64> {
    let mut total = 0u64;
    let mut included_any = false;
    for (included, value) in values {
        if !included {
            continue;
        }
        included_any = true;
        total = total.saturating_add(value?);
    }
    if included_any {
        Some(total)
    } else {
        Some(0)
    }
}

#[derive(Debug, Clone, Copy)]
struct FirefoxTabStyle {
    active: bool,
}

#[derive(Debug, Clone, Copy)]
struct IndicatorButtonStyle {
    color: Color,
}

impl iced::widget::button::StyleSheet for IndicatorButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: None,
            text_color: self.color,
            border: Border {
                color: Color::from_rgb8(0x00, 0x00, 0x00),
                width: 0.0,
                radius: 0.0.into(),
            },
            shadow_offset: Vector::new(0.0, 0.0),
            ..iced::widget::button::Appearance::default()
        }
    }
}

impl iced::widget::button::StyleSheet for FirefoxTabStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let palette = style.extended_palette();
        let background = if self.active {
            palette.background.base.color
        } else {
            palette.background.weak.color
        };
        let text_color = if self.active {
            palette.background.base.text
        } else {
            palette.background.weak.text
        };

        iced::widget::button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: [8.0, 8.0, 0.0, 0.0].into(),
            },
            shadow_offset: if self.active {
                Vector::new(0.0, 0.0)
            } else {
                Vector::new(0.0, 1.0)
            },
            ..iced::widget::button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let mut appearance = self.active(style);
        if !self.active {
            if let Some(Background::Color(color)) = appearance.background {
                let lifted = Color {
                    r: (color.r + 0.05).min(1.0),
                    g: (color.g + 0.05).min(1.0),
                    b: (color.b + 0.05).min(1.0),
                    a: color.a,
                };
                appearance.background = Some(Background::Color(lifted));
            }
        }
        appearance
    }
}

fn counter_oids_from_walk(varbinds: &[SnmpVarBind]) -> CounterOidSet {
    let mut seen = HashSet::new();
    let mut candidates: Vec<Oid> = varbinds
        .iter()
        .filter(|varbind| varbind.value.as_u64().is_some())
        .filter_map(|varbind| {
            if seen.insert(varbind.oid.clone()) {
                Some(varbind.oid.clone())
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));

    let mut mapping = CounterOidSet::default();
    let mut total = Vec::new();
    let mut total_seen = HashSet::new();

    for oid in &candidates {
        if oid.as_slice() == PRT_MARKER_LIFECOUNT_1.as_slice() {
            mapping.bw.push(oid.clone());
        }
        if oid.as_slice() == PRT_MARKER_LIFECOUNT_2.as_slice() {
            mapping.color.push(oid.clone());
        }
        if oid.as_slice() == PRT_MARKER_LIFECOUNT_3.as_slice() {
            if total_seen.insert(oid.clone()) {
                total.push(oid.clone());
            }
        }
    }

    for oid in candidates {
        if total_seen.insert(oid.clone()) {
            total.push(oid);
        }
    }

    mapping.total = total;
    mapping
}

fn snmp_oids(counter_oids: &CounterOidSet) -> Vec<Oid> {
    let mut oids = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |oid: Oid| {
        if seen.insert(oid.clone()) {
            oids.push(oid);
        }
    };

    push(Oid::from_slice(&SYS_DESCR_OID));
    push(Oid::from_slice(&SYS_OBJECT_ID_OID));
    push(Oid::from_slice(&SYS_NAME_OID));
    push(Oid::from_slice(&SYS_UPTIME_OID));
    push(Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID));
    push(Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID));
    push(Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID));
    push(Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID));
    push(Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID));

    for oid in &counter_oids.bw {
        push(oid.clone());
    }
    for oid in &counter_oids.color {
        push(oid.clone());
    }
    for oid in &counter_oids.total {
        push(oid.clone());
    }
    push(Oid::from_slice(&RICOH_TONER_BLACK_OID));
    push(Oid::from_slice(&RICOH_TONER_CYAN_OID));
    push(Oid::from_slice(&RICOH_TONER_MAGENTA_OID));
    push(Oid::from_slice(&RICOH_TONER_YELLOW_OID));

    oids
}

fn seed_printers() -> Vec<PrinterRecord> {
    Vec::new()
}
