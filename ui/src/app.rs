use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::time::Duration;

use iced::alignment::Horizontal;
use iced::keyboard;
use iced::theme;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Rule,
};
use iced::{Alignment, Application, Color, Command, Element, Length, Subscription, Theme};
use ron::de::from_str;
use ron::ser::{to_string_pretty, PrettyConfig};

use printcountpay_core::{
    default_discovery_cidr, probe_printer, resolve_counters, targets, CidrRange, CounterOidSet, Oid,
    PrinterId, PrinterRecord, SnmpAddress, SnmpConfig, SnmpRequest,
    SnmpV2cClient, SnmpVarBind, SnmpWalkRequest, DEFAULT_SNMP_PORT,
};

use crate::logging::{apply_log_level, LogEntry, LogLevel, LogStore, ReloadHandle};

mod constants;
mod badge_overlay;
mod helpers;
mod styles;
mod types;

pub use types::{Flags, Message, Tab, PrinterTab, RecordingCategory, SnmpErrorInfo, DiscoveryProbeResult, DiscoveryOutcome};

use constants::*;
use badge_overlay::BadgeOverlay;
use helpers::*;
use styles::*;
use types::*;

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

include!("app/views.rs");
include!("app/actions.rs");

