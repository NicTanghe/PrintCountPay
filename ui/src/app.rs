use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iced::alignment::Horizontal;
use iced::theme;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Application, Color, Command, Element, Length, Subscription, Theme};
use ron::de::from_str;
use ron::ser::{to_string_pretty, PrettyConfig};

use printcountpay_core::{
    resolve_counters, targets, CounterOidSet, Oid, PrinterId, PrinterRecord, PrinterStatus,
    SnmpAddress, SnmpConfig, SnmpRequest, SnmpResponse, SnmpV2cClient, SnmpVarBind,
    SnmpWalkRequest,
};

use crate::logging::{apply_log_level, LogEntry, LogLevel, LogStore, ReloadHandle};

const SYS_DESCR_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 1, 0];
const SYS_OBJECT_ID_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 2, 0];
const SYS_UPTIME_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 3, 0];
const PRT_MARKER_LIFECOUNT_ROOT: [u32; 11] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4];
const PRT_MARKER_LIFECOUNT_1: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 1];
const PRT_MARKER_LIFECOUNT_2: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 2];
const PRT_MARKER_LIFECOUNT_3: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Printers,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterTab {
    Polling,
    Oids,
}

#[derive(Debug, Clone)]
pub enum Message {
    LogTick,
    LogLevelChanged(LogLevel),
    ToggleTarget(String, bool),
    CopyDiagnostics,
    AddMockSnmp,
    SelectTab(Tab),
    SelectPrinterTab(PrinterTab),
    SelectPrinter(PrinterId),
    PollSelectedSnmp,
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
}

#[derive(Debug, Clone)]
pub struct SnmpErrorInfo {
    summary: String,
    detail: String,
}

#[derive(Debug, Clone)]
enum SnmpPollStatus {
    Idle,
    Polling,
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
    printers: Vec<PrinterRecord>,
    selected_printer: Option<PrinterId>,
    poll_states: HashMap<PrinterId, SnmpPollStatus>,
    poll_in_flight: HashSet<PrinterId>,
    snmp_config: SnmpConfig,
    counter_oids: CounterOidSet,
    oids_path: String,
    oids_bw_text: String,
    oids_color_text: String,
    oids_total_text: String,
    oids_status: Option<String>,
    oids_crawl_in_flight: bool,
}

impl Application for PrintCountApp {
    type Executor = iced::executor::Default;
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
                printers,
                selected_printer: None,
                poll_states,
                poll_in_flight: HashSet::new(),
                snmp_config: SnmpConfig::default(),
                counter_oids,
                oids_path: "counter_oids.ron".to_string(),
                oids_bw_text,
                oids_color_text,
                oids_total_text,
                oids_status: None,
                oids_crawl_in_flight: false,
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
            Message::PollSelectedSnmp => self.poll_selected_printer(),
            Message::SnmpPolled { printer_id, result } => {
                self.poll_in_flight.remove(&printer_id);
                let received_at = now_epoch_seconds();
                let state = match result {
                    Ok(response) => SnmpPollStatus::Ok {
                        received_at,
                        varbinds: response.varbinds,
                    },
                    Err(error) => SnmpPollStatus::Error {
                        received_at,
                        summary: error.summary,
                        detail: error.detail,
                    },
                };
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
                        let count = set.bw.len() + set.color.len() + set.total.len();
                        self.counter_oids = set;
                        self.sync_oid_inputs();
                        self.oids_status = Some(format!(
                            "Crawl mapped {count} OIDs (index order)."
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
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let log_tick = iced::time::every(Duration::from_millis(250)).map(|_| Message::LogTick);
        let poll_tick = iced::time::every(Duration::from_secs(5)).map(|_| Message::PollSelectedSnmp);
        Subscription::batch(vec![log_tick, poll_tick])
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
            self.printer_tab_button(PrinterTab::Oids, "SNMP OIDs")
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn printer_tab_button(&self, tab: PrinterTab, label: &str) -> Element<'_, Message> {
        let style = if self.printer_tab == tab {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        };

        button(text(label))
            .style(style)
            .on_press(Message::SelectPrinterTab(tab))
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

    fn printer_list_view(&self) -> Element<'_, Message> {
        let mut list_items = column![].spacing(6);

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            for record in &self.printers {
                list_items = list_items.push(self.printer_row(record));
            }
        }

        let scroll = scrollable(list_items)
            .height(Length::Fill)
            .width(Length::Fill);

        let content = column![
            text("Found printers")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Demo list (discovery not wired yet).")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            scroll
        ]
        .spacing(8);

        container(content)
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
            .unwrap_or("unknown host");
        let model = record.model.as_deref().unwrap_or("Unknown model");
        let status = status_label(record.status);

        let content = column![
            text(address)
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
            text(model)
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
        let Some(selected) = &self.selected_printer else {
            let content = column![
                text("SNMP polling")
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Select a printer to start polling.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            ]
            .spacing(8);

            return container(content)
                .padding(12)
                .width(Length::FillPortion(2))
                .height(Length::Fill)
                .style(theme::Container::Box)
                .into();
        };

        let Some(record) = self.printers.iter().find(|record| &record.id == selected) else {
            let content = column![
                text("SNMP polling")
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Selected printer not found.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            ]
            .spacing(8);

            return container(content)
                .padding(12)
                .width(Length::FillPortion(2))
                .height(Length::Fill)
                .style(theme::Container::Box)
                .into();
        };

        let address = record
            .snmp_address
            .as_ref()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not set".to_string());
        let model = record.model.as_deref().unwrap_or("Unknown model");
        let state = self
            .poll_states
            .get(&record.id)
            .cloned()
            .unwrap_or(SnmpPollStatus::Idle);

        let header = column![
            text("Printer details")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text(format!("ID: {}", record.id))
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            text(format!("Address: {}", address))
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            text(format!("Model: {}", model))
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
        ]
        .spacing(4);

        let body = match self.printer_tab {
            PrinterTab::Polling => self.printer_poll_view(&state),
            PrinterTab::Oids => self.printer_oids_view(record),
        };

        let content = column![header, self.printer_tab_bar(), body].spacing(12);

        container(content)
            .padding(12)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_poll_view(&self, state: &SnmpPollStatus) -> Element<'_, Message> {
        let content = column![
            text("Polling every 5 seconds")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            self.poll_state_view(state),
            self.counters_view(state),
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
            text("Crawl root: 1.3.6.1.2.1.43.10.2.1.4")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        content.into()
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

    fn poll_state_view(&self, state: &SnmpPollStatus) -> Element<'_, Message> {
        match state {
            SnmpPollStatus::Idle => text("Waiting for next poll.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into(),
            SnmpPollStatus::Polling => text("Polling SNMP...")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x3b, 0x82, 0xf6)))
                .into(),
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let mut rows = column![].spacing(4);
                if varbinds.is_empty() {
                    rows = rows.push(
                        text("No varbinds returned.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                } else {
                    for varbind in varbinds {
                        rows = rows.push(
                            text(format!("{} = {}", varbind.oid, varbind.value))
                                .size(13)
                                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
                        );
                    }
                }

                let list = scrollable(rows)
                    .height(Length::Fill)
                    .width(Length::Fill);

                column![
                    text(format!("Last poll: {}", received_at))
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    list
                ]
                .spacing(6)
                .into()
            }
            SnmpPollStatus::Error {
                received_at,
                summary,
                detail,
            } => column![
                text(format!("Last poll: {}", received_at))
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                text(format!("Error: {}", summary))
                    .size(13)
                    .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
                text(detail)
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(4)
            .into(),
        }
    }

    fn counters_view(&self, state: &SnmpPollStatus) -> Element<'_, Message> {
        let header = text("Counters")
            .size(18)
            .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)));

        let body: Element<'_, Message> = match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let resolution = resolve_counters(*received_at, &self.counter_oids, varbinds);
                let mut lines = column![
                    self.counter_line("B/W clicks", resolution.snapshot.bw),
                    self.counter_line("Color clicks", resolution.snapshot.color),
                    self.counter_line("Total clicks", resolution.snapshot.total),
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
            SnmpPollStatus::Polling => text("Polling counters...")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3b, 0x82, 0xf6)))
                .into(),
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
            .insert(printer_id.clone(), SnmpPollStatus::Polling);

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

        let mut request =
            SnmpWalkRequest::new(address, Oid::from_slice(&PRT_MARKER_LIFECOUNT_ROOT));
        if let Some(community) = record.community.clone() {
            request = request.with_community(community);
        }

        let config = self.snmp_config.clone();
        self.oids_crawl_in_flight = true;
        self.oids_status = Some("Crawling prtMarkerLifeCount...".to_string());

        Command::perform(
            async move {
                let client = SnmpV2cClient::new(config);
                match client.walk(request).await {
                    Ok(response) => Ok(counter_oids_from_walk(&response.varbinds)),
                    Err(error) => Err(SnmpErrorInfo {
                        summary: error.user_summary(),
                        detail: error.technical_detail(),
                    }),
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
        bw: vec![Oid::from_slice(&PRT_MARKER_LIFECOUNT_1)],
        color: vec![Oid::from_slice(&PRT_MARKER_LIFECOUNT_2)],
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

fn oid_is_descendant(root: &[u32], candidate: &Oid) -> bool {
    let candidate = candidate.as_slice();
    candidate.len() >= root.len() && candidate[..root.len()] == root[..]
}

fn counter_oids_from_walk(varbinds: &[SnmpVarBind]) -> CounterOidSet {
    let mut candidates: Vec<Oid> = varbinds
        .iter()
        .map(|varbind| varbind.oid.clone())
        .filter(|oid| oid_is_descendant(&PRT_MARKER_LIFECOUNT_ROOT, oid))
        .collect();
    candidates.sort_by(|left, right| left.as_slice().cmp(right.as_slice()));

    let mut mapping = CounterOidSet::default();
    let mut used = HashSet::new();

    if let Some(oid) = candidates
        .iter()
        .find(|oid| oid.as_slice() == PRT_MARKER_LIFECOUNT_1.as_slice())
    {
        mapping.bw.push(oid.clone());
        used.insert(oid.clone());
    }
    if let Some(oid) = candidates
        .iter()
        .find(|oid| oid.as_slice() == PRT_MARKER_LIFECOUNT_2.as_slice())
    {
        mapping.color.push(oid.clone());
        used.insert(oid.clone());
    }
    if let Some(oid) = candidates
        .iter()
        .find(|oid| oid.as_slice() == PRT_MARKER_LIFECOUNT_3.as_slice())
    {
        mapping.total.push(oid.clone());
        used.insert(oid.clone());
    }

    let mut fallback = candidates.into_iter().filter(|oid| !used.contains(oid));
    if mapping.bw.is_empty() {
        if let Some(oid) = fallback.next() {
            mapping.bw.push(oid);
        }
    }
    if mapping.color.is_empty() {
        if let Some(oid) = fallback.next() {
            mapping.color.push(oid);
        }
    }
    if mapping.total.is_empty() {
        if let Some(oid) = fallback.next() {
            mapping.total.push(oid);
        }
    }

    mapping
}

fn snmp_oids(counter_oids: &CounterOidSet) -> Vec<Oid> {
    let mut oids = vec![
        Oid::from_slice(&SYS_DESCR_OID),
        Oid::from_slice(&SYS_OBJECT_ID_OID),
        Oid::from_slice(&SYS_UPTIME_OID),
    ];

    oids.extend(counter_oids.bw.iter().cloned());
    oids.extend(counter_oids.color.iter().cloned());
    oids.extend(counter_oids.total.iter().cloned());

    oids
}

fn seed_printers() -> Vec<PrinterRecord> {
    vec![
        PrinterRecord {
            id: PrinterId::new("demo-ricoh-1"),
            ip_or_hostname: Some("192.168.1.10".to_string()),
            model: Some("Ricoh IM C3000".to_string()),
            sys_object_id: None,
            snmp_address: Some(SnmpAddress::with_default_port("192.168.1.10")),
            community: None,
            status: PrinterStatus::Unknown,
            last_seen: None,
        },
        PrinterRecord {
            id: PrinterId::new("demo-ricoh-2"),
            ip_or_hostname: Some("192.168.1.11".to_string()),
            model: Some("Ricoh IM 4000".to_string()),
            sys_object_id: None,
            snmp_address: Some(SnmpAddress::with_default_port("192.168.1.11")),
            community: None,
            status: PrinterStatus::Unknown,
            last_seen: None,
        },
    ]
}
