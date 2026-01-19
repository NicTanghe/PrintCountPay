use printcountpay_core::{CounterOidSet, PrinterId, PrinterRecord, SnmpResponse, SnmpVarBind};

use crate::logging::{LogLevel, LogStore, ReloadHandle};

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
    pub(crate) summary: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone)]
pub(crate) enum SnmpPollStatus {
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
pub(crate) struct RecordingSnapshot {
    pub(crate) received_at: u64,
    pub(crate) bw_printer: Option<u64>,
    pub(crate) bw_copier: Option<u64>,
    pub(crate) color_printer: Option<u64>,
    pub(crate) color_copier: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordingCategory {
    CopiesBw,
    CopiesColor,
    PrintsBw,
    PrintsColor,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordingCategoryEdits {
    pub(crate) include_in_price: bool,
    pub(crate) start_input: String,
    pub(crate) end_input: String,
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
pub(crate) struct RecordingEdits {
    pub(crate) copies_bw: RecordingCategoryEdits,
    pub(crate) copies_color: RecordingCategoryEdits,
    pub(crate) prints_bw: RecordingCategoryEdits,
    pub(crate) prints_color: RecordingCategoryEdits,
}

impl RecordingEdits {
    pub(crate) fn category(&self, category: RecordingCategory) -> &RecordingCategoryEdits {
        match category {
            RecordingCategory::CopiesBw => &self.copies_bw,
            RecordingCategory::CopiesColor => &self.copies_color,
            RecordingCategory::PrintsBw => &self.prints_bw,
            RecordingCategory::PrintsColor => &self.prints_color,
        }
    }

    pub(crate) fn category_mut(
        &mut self,
        category: RecordingCategory,
    ) -> &mut RecordingCategoryEdits {
        match category {
            RecordingCategory::CopiesBw => &mut self.copies_bw,
            RecordingCategory::CopiesColor => &mut self.copies_color,
            RecordingCategory::PrintsBw => &mut self.prints_bw,
            RecordingCategory::PrintsColor => &mut self.prints_color,
        }
    }

    pub(crate) fn apply_start_snapshot(&mut self, snapshot: &RecordingSnapshot) {
        set_input(&mut self.copies_bw.start_input, snapshot.bw_copier);
        set_input(&mut self.copies_color.start_input, snapshot.color_copier);
        set_input(&mut self.prints_bw.start_input, snapshot.bw_printer);
        set_input(&mut self.prints_color.start_input, snapshot.color_printer);
        self.clear_end_inputs();
    }

    pub(crate) fn apply_end_snapshot(&mut self, snapshot: &RecordingSnapshot) {
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

fn set_input(target: &mut String, value: Option<u64>) {
    target.clear();
    if let Some(value) = value {
        target.push_str(&value.to_string());
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RecordingSession {
    pub(crate) active: bool,
    pub(crate) start: Option<RecordingSnapshot>,
    pub(crate) end: Option<RecordingSnapshot>,
    pub(crate) status: Option<String>,
    pub(crate) edits: RecordingEdits,
}

#[derive(Debug, Clone)]
pub(crate) struct PricingSettings {
    pub(crate) bw_first_input: String,
    pub(crate) bw_next_input: String,
    pub(crate) bw_rest_input: String,
    pub(crate) color_input: String,
    pub(crate) round_to_half_euro: bool,
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
pub(crate) struct BwPricing {
    pub(crate) first_cents: u64,
    pub(crate) next_cents: u64,
    pub(crate) rest_cents: u64,
}

#[derive(Debug, Clone)]
pub struct DiscoveryProbeResult {
    pub(crate) run_id: u64,
    pub(crate) outcome: DiscoveryOutcome,
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
