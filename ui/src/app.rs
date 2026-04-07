use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::time::Duration;

use iced::alignment::Horizontal;
use iced::keyboard;
use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, pick_list, row, rule, scrollable, text,
    text_input,
};
use iced::{Alignment, Color, Element, Length, Subscription, Task as Command, Theme, window};
use ron::de::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};

use printcountpay_core::{
    CidrRange, CounterOidSet, DEFAULT_SNMP_PORT, Oid, PrinterId, PrinterRecord, PrinterStatus,
    SnmpAddress, SnmpConfig, SnmpRequest, SnmpResponse, SnmpV2cClient, SnmpVarBind,
    SnmpWalkRequest, default_discovery_cidr, probe_printer, resolve_counters, targets,
    varbind_display_value, varbind_numeric_value, varbind_text_value,
};

use crate::logging::{LogEntry, LogLevel, LogStore, ReloadHandle, apply_log_level};
use crate::sync::{self, SharedState, SyncCommand, SyncEvent, SyncRole};

mod badge_overlay;
mod constants;
mod helpers;
mod paths;
mod profiles;
mod styles;
mod types;

pub use types::{
    DiscoveryOutcome, DiscoveryProbeResult, Flags, ManualBwTier, ManualColorTier,
    ManualModifierChoice, ManualPaperModifier, ManualPricingLineItem, ManualPricingSettings,
    ManualPricingTab, ManualPrintMode, ManualPrintSize, ManualRoundingMode, Message, PrinterTab,
    ProfileChoice, RecordingCategory, SnmpErrorInfo, Tab,
};
pub(crate) use types::{PricingSettings, RecordingSession, SnmpPollStatus};

use badge_overlay::BadgeOverlay;
use constants::*;
use helpers::*;
use paths::*;
use profiles::*;
use styles::*;
use types::*;

fn merge_status_messages(primary: Option<String>, secondary: Option<String>) -> Option<String> {
    match (primary, secondary) {
        (Some(primary), Some(secondary)) => Some(format!("{primary} | {secondary}")),
        (Some(primary), None) => Some(primary),
        (None, Some(secondary)) => Some(secondary),
        (None, None) => None,
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub struct PrintCountApp {
    log_store: LogStore,
    reload_handle: ReloadHandle,
    log_entries: Vec<LogEntry>,
    log_level: LogLevel,
    known_targets: HashSet<String>,
    enabled_targets: HashSet<String>,
    copy_status: Option<String>,
    advanced_mode: bool,
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
    manual_pricing_path: String,
    manual_pricing_status: Option<String>,
    printers_path: String,
    printers_status: Option<String>,
    printers: Vec<PrinterRecord>,
    selected_printer: Option<PrinterId>,
    manual_pricing_selected: bool,
    manual_pricing_tab: ManualPricingTab,
    manual_pricing: ManualPricingSettings,
    poll_states: HashMap<PrinterId, SnmpPollStatus>,
    poll_in_flight: HashSet<PrinterId>,
    poll_export_path: String,
    poll_export_status: Option<String>,
    snmp_config: SnmpConfig,
    counter_oids: CounterOidSet,
    data_root: String,
    profiles_root: String,
    profile_index: ProfileIndex,
    profile_status: Option<String>,
    active_profile: Option<ManufacturerProfile>,
    oids_path: String,
    oids_total_text: String,
    oids_status: Option<String>,
    oids_crawl_in_flight: bool,
    recording_oids: RecordingOidSettings,
    recording_sessions: HashMap<PrinterId, RecordingSession>,
    pricing: PricingSettings,
    sync_sender: Option<tokio::sync::mpsc::UnboundedSender<SyncCommand>>,
    sync_role: SyncRole,
    sync_status_detail: String,
    last_shared_state: SharedState,
}

impl PrintCountApp {
    pub(crate) fn new(flags: Flags) -> (Self, Command<Message>) {
        let default_targets = [
            targets::DISCOVERY,
            targets::SNMP,
            targets::POLLING,
            targets::UI,
            targets::STORAGE,
        ];
        let known_targets: HashSet<String> = default_targets
            .iter()
            .map(|value| value.to_string())
            .collect();
        let enabled_targets = known_targets.clone();
        let printers: Vec<PrinterRecord> = Vec::new();
        let AppPaths {
            data_root,
            profiles_root,
            printers_file,
            counter_oids_file,
            poll_export_file,
            status: path_status,
        } = resolve_app_paths();
        let data_root = display_path(&data_root);
        let profiles_root = display_path(&profiles_root);
        let (profile_index, profile_status) = load_profile_index(Path::new(&profiles_root));
        let profile_status = merge_status_messages(path_status, profile_status);
        let counter_oids = default_counter_oids();
        let oids_total_text = format_oid_list(&counter_oids.total);
        let recording_oids = default_recording_oid_inputs();
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

        let mut app = Self {
            log_store: flags.log_store,
            reload_handle: flags.reload_handle,
            log_entries: Vec::new(),
            log_level: LogLevel::default(),
            known_targets,
            enabled_targets,
            copy_status: None,
            advanced_mode: false,
            active_tab: Tab::Printers,
            printer_tab: PrinterTab::Recording,
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
            manual_pricing_path: String::new(),
            manual_pricing_status: None,
            printers_path: display_path(&printers_file),
            printers_status: None,
            printers,
            selected_printer: None,
            manual_pricing_selected: false,
            manual_pricing_tab: ManualPricingTab::Calculator,
            manual_pricing: ManualPricingSettings::default(),
            poll_states,
            poll_in_flight: HashSet::new(),
            poll_export_path: display_path(&poll_export_file),
            poll_export_status: None,
            snmp_config: SnmpConfig::default(),
            counter_oids,
            data_root,
            profiles_root,
            profile_index,
            profile_status,
            active_profile: None,
            oids_path: display_path(&counter_oids_file),
            oids_total_text,
            oids_status: None,
            oids_crawl_in_flight: false,
            recording_oids,
            recording_sessions: HashMap::new(),
            pricing: PricingSettings::default(),
            sync_sender: None,
            sync_role: SyncRole::Searching,
            sync_status_detail: format!(
                "Searching for sync host on UDP {} / TCP {}.",
                sync::SYNC_DISCOVERY_PORT,
                sync::SYNC_PORT
            ),
            last_shared_state: SharedState::default(),
        };
        if !app.advanced_mode {
            app.printers_path = app.default_printers_path();
            app.load_printers_if_present();
        }
        app.manual_pricing_path = app.default_manual_pricing_path();
        app.load_manual_pricing_if_present();
        app.last_shared_state = app.build_shared_state(app.last_shared_state.revision);

        (app, Command::none())
    }

    pub(crate) fn title(&self) -> String {
        "Ricoh PrintCount".to_string()
    }

    pub(crate) fn update(&mut self, message: Message) -> Command<Message> {
        let command = match message {
            Message::LogTick => {
                self.refresh_logs();
                Command::none()
            }
            Message::SyncTick => {
                self.flush_shared_state();
                Command::none()
            }
            Message::SyncEvent(event) => self.handle_sync_event(event),
            Message::ToggleAdvancedMode => {
                self.advanced_mode = !self.advanced_mode;
                if !self.advanced_mode {
                    self.active_tab = Tab::Printers;
                    if !matches!(
                        self.printer_tab,
                        PrinterTab::Recording | PrinterTab::Pricing
                    ) {
                        self.printer_tab = PrinterTab::Recording;
                    }
                    self.printers_path = self.default_printers_path();
                    self.load_printers_if_present();
                }
                Command::none()
            }
            Message::DragWindow => window::latest().and_then(window::drag),
            Message::MinimizeWindow => window::latest().and_then(|id| window::minimize(id, true)),
            Message::CloseWindow => window::latest().and_then(window::close),
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
                if self.advanced_mode || tab == Tab::Printers {
                    self.active_tab = tab;
                }
                Command::none()
            }
            Message::SelectManualPricing => {
                self.active_tab = Tab::Printers;
                self.manual_pricing_selected = true;
                Command::none()
            }
            Message::SelectManualPricingTab(tab) => {
                self.manual_pricing_tab = tab;
                Command::none()
            }
            Message::SelectPrinterTab(tab) => {
                if self.advanced_mode || matches!(tab, PrinterTab::Recording | PrinterTab::Pricing)
                {
                    self.printer_tab = tab;
                }
                Command::none()
            }
            Message::SelectPrinter(printer_id) => {
                self.manual_pricing_selected = false;
                self.selected_printer = Some(printer_id.clone());
                self.apply_profile_for_printer(&printer_id, None);
                self.poll_selected_printer()
            }
            Message::ProfileChoiceChanged(choice) => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    if let Some(record) = self
                        .printers
                        .iter_mut()
                        .find(|record| record.id == printer_id)
                    {
                        record.profile_id = match choice {
                            ProfileChoice::Auto => None,
                            ProfileChoice::Profile(id) => Some(id),
                        };
                    }
                    self.apply_profile_for_printer(&printer_id, None);
                }
                Command::none()
            }
            Message::DeleteSelectedPrinter => {
                self.delete_selected_printer();
                Command::none()
            }
            Message::PollSelectedSnmp => self.poll_selected_printer(),
            Message::PollPrinterById(printer_id) => self.poll_printer(printer_id),
            Message::PollExportPathChanged(value) => {
                self.poll_export_path = value;
                Command::none()
            }
            Message::ExportPollData => {
                self.export_poll_data();
                Command::none()
            }
            Message::SnmpPolled { printer_id, result } => {
                self.handle_snmp_polled(printer_id, result);
                Command::none()
            }
            Message::OidsPathChanged(value) => {
                self.oids_path = value;
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
            Message::RecordingOidCopiesBwChanged(value) => {
                self.recording_oids.copies_bw_input = value;
                Command::none()
            }
            Message::RecordingOidCopiesColorChanged(value) => {
                self.recording_oids.copies_color_input = value;
                Command::none()
            }
            Message::RecordingOidPrintsBwChanged(value) => {
                self.recording_oids.prints_bw_input = value;
                Command::none()
            }
            Message::RecordingOidPrintsColorChanged(value) => {
                self.recording_oids.prints_color_input = value;
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
                    let session = self.recording_sessions.entry(printer_id).or_default();
                    session.edits.category_mut(category).start_input = value;
                }
                Command::none()
            }
            Message::RecordingEndChanged { category, value } => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self.recording_sessions.entry(printer_id).or_default();
                    session.edits.category_mut(category).end_input = value;
                }
                Command::none()
            }
            Message::RecordingEndResetToPolled(category) => {
                self.reset_recording_end_to_polled(category);
                Command::none()
            }
            Message::RecordingToggleInclude(category) => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self.recording_sessions.entry(printer_id).or_default();
                    let entry = session.edits.category_mut(category);
                    entry.include_in_price = !entry.include_in_price;
                }
                Command::none()
            }
            Message::RecordingEndFieldsUnlockedChanged(value) => {
                if let Some(printer_id) = self.selected_printer.clone() {
                    let session = self.recording_sessions.entry(printer_id).or_default();
                    session.end_fields_unlocked = value;
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
                self.pricing.round_to_five_cents = value;
                Command::none()
            }
            Message::ManualPricingLineAdded => {
                self.manual_pricing
                    .line_items
                    .push(ManualPricingLineItem::default());
                Command::none()
            }
            Message::ManualPricingLineRemoved(index) => {
                if self.manual_pricing.line_items.len() > 1 {
                    if index < self.manual_pricing.line_items.len() {
                        self.manual_pricing.line_items.remove(index);
                    }
                } else if let Some(line_item) = self.manual_pricing.line_items.first_mut() {
                    *line_item = ManualPricingLineItem::default();
                } else {
                    self.manual_pricing
                        .line_items
                        .push(ManualPricingLineItem::default());
                }
                Command::none()
            }
            Message::ManualPricingLineSizeChanged(index, size) => {
                if let Some(line_item) = self.manual_pricing.line_items.get_mut(index) {
                    line_item.size = size;
                    if line_item.modifier_index.is_some_and(|modifier_index| {
                        self.manual_pricing
                            .modifiers
                            .get(modifier_index)
                            .map(|modifier| !modifier.applies_to_size(size))
                            .unwrap_or(true)
                    }) {
                        line_item.modifier_index = None;
                    }
                }
                Command::none()
            }
            Message::ManualPricingLinePrintModeChanged(index, print_mode) => {
                if let Some(line_item) = self.manual_pricing.line_items.get_mut(index) {
                    line_item.print_mode = print_mode;
                }
                Command::none()
            }
            Message::ManualPricingLineModifierChanged(index, modifier_index) => {
                if let Some(line_item) = self.manual_pricing.line_items.get_mut(index) {
                    line_item.modifier_index = modifier_index;
                }
                Command::none()
            }
            Message::ManualPricingLineSidesChanged(index, value) => {
                if let Some(line_item) = self.manual_pricing.line_items.get_mut(index) {
                    line_item.sides_input = value;
                    line_item.sync_sheets_from_sides();
                }
                Command::none()
            }
            Message::ManualPricingLineDoubleSidedChanged(index, value) => {
                if let Some(line_item) = self.manual_pricing.line_items.get_mut(index) {
                    line_item.double_sided = value;
                    line_item.sync_sheets_from_sides();
                }
                Command::none()
            }
            Message::ManualPricingBasePriceChanged(size, value) => {
                self.manual_pricing.set_size_price_input(size, value);
                Command::none()
            }
            Message::ManualPricingBwTierChanged(size, tier, value) => {
                self.manual_pricing.set_bw_tier_input(size, tier, value);
                Command::none()
            }
            Message::ManualPricingColorTierChanged(size, tier, value) => {
                self.manual_pricing.set_color_tier_input(size, tier, value);
                Command::none()
            }
            Message::ManualPricingModifierAdded => {
                self.manual_pricing
                    .modifiers
                    .insert(0, ManualPaperModifier::default());
                for line_item in &mut self.manual_pricing.line_items {
                    if let Some(selected) = line_item.modifier_index {
                        line_item.modifier_index = Some(selected + 1);
                    }
                }
                Command::none()
            }
            Message::ManualPricingModifierRemoved(index) => {
                if self.manual_pricing.modifiers.len() > 1 {
                    if index < self.manual_pricing.modifiers.len() {
                        self.manual_pricing.modifiers.remove(index);
                        for line_item in &mut self.manual_pricing.line_items {
                            match line_item.modifier_index {
                                Some(selected) if selected == index => line_item.modifier_index = None,
                                Some(selected) if selected > index => {
                                    line_item.modifier_index = Some(selected - 1);
                                }
                                _ => {}
                            }
                        }
                    }
                } else if let Some(modifier) = self.manual_pricing.modifiers.first_mut() {
                    *modifier = ManualPaperModifier::default();
                    for line_item in &mut self.manual_pricing.line_items {
                        line_item.modifier_index = None;
                    }
                } else {
                    self.manual_pricing
                        .modifiers
                        .push(ManualPaperModifier::default());
                }
                Command::none()
            }
            Message::ManualPricingModifierNameChanged(index, value) => {
                if let Some(modifier) = self.manual_pricing.modifiers.get_mut(index) {
                    modifier.name_input = value;
                }
                Command::none()
            }
            Message::ManualPricingModifierPriceChanged(index, size, value) => {
                if let Some(modifier) = self.manual_pricing.modifiers.get_mut(index) {
                    modifier.set_price_input(size, value);
                }
                Command::none()
            }
            Message::ManualPricingModifierAppliesChanged(index, size, value) => {
                if let Some(modifier) = self.manual_pricing.modifiers.get_mut(index) {
                    modifier.set_applies_to_size(size, value);
                    if !value {
                        for line_item in &mut self.manual_pricing.line_items {
                            if line_item.size == size && line_item.modifier_index == Some(index) {
                                line_item.modifier_index = None;
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::ManualPricingPathChanged(value) => {
                self.manual_pricing_path = value;
                Command::none()
            }
            Message::LoadManualPricing => {
                self.load_manual_pricing_from_path();
                Command::none()
            }
            Message::SaveManualPricing => {
                self.save_manual_pricing_to_path();
                Command::none()
            }
            Message::ManualPricingCuttingChanged(value) => {
                self.manual_pricing.cutting_enabled = value;
                Command::none()
            }
            Message::ManualPricingDiscountChanged(value) => {
                self.manual_pricing.discount_input = value;
                Command::none()
            }
            Message::ManualPricingRoundingToggled(mode, enabled) => {
                self.manual_pricing.rounding_mode = if enabled {
                    mode
                } else if self.manual_pricing.rounding_mode == mode {
                    ManualRoundingMode::None
                } else {
                    self.manual_pricing.rounding_mode
                };
                Command::none()
            }
        };

        self.flush_shared_state();
        command
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let log_tick = iced::time::every(Duration::from_millis(250)).map(|_| Message::LogTick);
        let poll_tick =
            iced::time::every(Duration::from_secs(5)).map(|_| Message::PollSelectedSnmp);
        let sync_tick = iced::time::every(sync::SYNC_FLUSH_INTERVAL).map(|_| Message::SyncTick);
        let sync_subscription = sync::subscription().map(Message::SyncEvent);
        let delete_key = iced::event::listen_with(|event, _status, _window| match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                delete_key_event(key.clone(), modifiers)
            }
            _ => None,
        });
        Subscription::batch(vec![
            log_tick,
            poll_tick,
            sync_tick,
            sync_subscription,
            delete_key,
        ])
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let sidebar = container(self.printer_list_view())
            .width(Length::Fixed(367.0))
            .height(Length::Fill);
        let main_content = if self.advanced_mode && self.active_tab == Tab::Debug {
            self.debug_tab_view()
        } else if self.manual_pricing_selected {
            self.manual_pricing_panel_view()
        } else {
            self.printer_details_view()
        };
        let top_area = mouse_area(self.window_controls_bar()).on_press(Message::DragWindow);
        let right_column = column![top_area, main_content]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);
        let right_panel = container(right_column)
            .padding(14)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(theme::Container::Custom(right_panel_style()));

        let content = row![sidebar, right_panel]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        let shell = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .style(theme::Container::Custom(window_shell_style()));

        container(shell)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn horizontal_space() -> Space {
    Space::new().width(Length::Fill)
}

trait AlignItemsExt: Sized {
    fn align_items(self, alignment: Alignment) -> Self;
}

impl<'a, Message, ThemeT, Renderer> AlignItemsExt
    for iced::widget::Row<'a, Message, ThemeT, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn align_items(self, alignment: Alignment) -> Self {
        self.align_y(alignment)
    }
}

impl<'a, Message, ThemeT, Renderer> AlignItemsExt
    for iced::widget::Column<'a, Message, ThemeT, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn align_items(self, alignment: Alignment) -> Self {
        self.align_x(alignment)
    }
}

trait TextCompatExt: Sized {
    fn horizontal_alignment(self, alignment: Horizontal) -> Self;
}

impl<'a, ThemeT, Renderer> TextCompatExt for iced::widget::Text<'a, ThemeT, Renderer>
where
    ThemeT: iced::widget::text::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    fn horizontal_alignment(self, alignment: Horizontal) -> Self {
        self.align_x(alignment)
    }
}

include!("app/views.rs");
include!("app/actions.rs");
