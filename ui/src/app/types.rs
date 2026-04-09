use printcountpay_core::{
    CounterOidSet, Error as CoreError, PrinterId, PrinterRecord, PrinterStatus, SnmpResponse,
    SnmpVarBind,
};
use serde::{Deserialize, Serialize};

use crate::logging::{LogLevel, LogStore, ReloadHandle};
use crate::sync::SyncEvent;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualPricingTab {
    Calculator,
    Prices,
    Finishers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManualPrintSize {
    A0,
    A1,
    A2,
    A3,
    A4,
}

impl ManualPrintSize {
    pub const ALL: [Self; 5] = [Self::A0, Self::A1, Self::A2, Self::A3, Self::A4];
}

impl std::fmt::Display for ManualPrintSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A0 => write!(f, "A0"),
            Self::A1 => write!(f, "A1"),
            Self::A2 => write!(f, "A2"),
            Self::A3 => write!(f, "A3"),
            Self::A4 => write!(f, "A4"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ManualPrintMode {
    #[default]
    Bw,
    Color,
}

impl ManualPrintMode {
    pub const ALL: [Self; 2] = [Self::Bw, Self::Color];
}

impl std::fmt::Display for ManualPrintMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bw => write!(f, "B/W"),
            Self::Color => write!(f, "Color"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ManualFinisherType {
    #[default]
    Laminate,
    Folding,
    Binding,
}

impl ManualFinisherType {
    pub const ALL: [Self; 3] = [Self::Laminate, Self::Folding, Self::Binding];
}

impl std::fmt::Display for ManualFinisherType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Laminate => write!(f, "Laminate"),
            Self::Folding => write!(f, "Folding"),
            Self::Binding => write!(f, "Binding"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ManualLaminateSize {
    A0,
    A1,
    A2,
    A3,
    #[default]
    A4,
    A5,
}

impl ManualLaminateSize {
    pub const ALL: [Self; 6] = [Self::A0, Self::A1, Self::A2, Self::A3, Self::A4, Self::A5];
}

impl std::fmt::Display for ManualLaminateSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A0 => write!(f, "A0"),
            Self::A1 => write!(f, "A1"),
            Self::A2 => write!(f, "A2"),
            Self::A3 => write!(f, "A3"),
            Self::A4 => write!(f, "A4"),
            Self::A5 => write!(f, "A5"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualBwTier {
    FirstFive,
    NextFive,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualColorTier {
    FirstFive,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ManualRoundingMode {
    #[default]
    FiveCents,
    None,
    HalfEuro,
    DownToFiveEuro,
    DownToTenEuro,
}

impl std::fmt::Display for ManualRoundingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "No rounding"),
            Self::FiveCents => write!(f, "Round down to 0.05 EUR"),
            Self::HalfEuro => write!(f, "Round down to 0.50 EUR"),
            Self::DownToFiveEuro => write!(f, "Round down to 5 EUR"),
            Self::DownToTenEuro => write!(f, "Round down to 10 EUR"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPricingLineItem {
    pub(crate) size: ManualPrintSize,
    #[serde(default)]
    pub(crate) print_mode: ManualPrintMode,
    #[serde(default)]
    pub(crate) modifier_index: Option<usize>,
    #[serde(default)]
    pub(crate) double_sided: bool,
    pub(crate) sheets_input: String,
    pub(crate) sides_input: String,
}

impl Default for ManualPricingLineItem {
    fn default() -> Self {
        Self {
            size: ManualPrintSize::A3,
            print_mode: ManualPrintMode::Color,
            modifier_index: None,
            double_sided: false,
            sheets_input: "0".to_string(),
            sides_input: "0".to_string(),
        }
    }
}

impl ManualPricingLineItem {
    pub(crate) fn derived_sheets(&self) -> Option<u64> {
        let trimmed = self.sides_input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let sides = trimmed.parse::<u64>().ok()?;
        Some(if self.double_sided {
            (sides + 1) / 2
        } else {
            sides
        })
    }

    pub(crate) fn sync_sheets_from_sides(&mut self) {
        self.sheets_input = self
            .derived_sheets()
            .map(|value| value.to_string())
            .unwrap_or_default();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualFinisherLineItem {
    #[serde(default)]
    pub(crate) finisher_type: ManualFinisherType,
    #[serde(default)]
    pub(crate) laminate_size: ManualLaminateSize,
    pub(crate) amount_input: String,
}

impl Default for ManualFinisherLineItem {
    fn default() -> Self {
        Self {
            finisher_type: ManualFinisherType::Laminate,
            laminate_size: ManualLaminateSize::A4,
            amount_input: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPaperModifier {
    pub(crate) name_input: String,
    #[serde(default, rename = "price_input", skip_serializing)]
    pub(crate) legacy_price_input: String,
    #[serde(default)]
    pub(crate) applies_a0: bool,
    #[serde(default)]
    pub(crate) a0_price_input: String,
    #[serde(default)]
    pub(crate) applies_a1: bool,
    #[serde(default)]
    pub(crate) a1_price_input: String,
    #[serde(default)]
    pub(crate) applies_a2: bool,
    #[serde(default)]
    pub(crate) a2_price_input: String,
    #[serde(default)]
    pub(crate) applies_a3: bool,
    #[serde(default)]
    pub(crate) a3_price_input: String,
    #[serde(default)]
    pub(crate) applies_a4: bool,
    #[serde(default)]
    pub(crate) a4_price_input: String,
}

impl ManualPaperModifier {
    pub(crate) fn applies_to_size(&self, size: ManualPrintSize) -> bool {
        match size {
            ManualPrintSize::A0 => self.applies_a0,
            ManualPrintSize::A1 => self.applies_a1,
            ManualPrintSize::A2 => self.applies_a2,
            ManualPrintSize::A3 => self.applies_a3,
            ManualPrintSize::A4 => self.applies_a4,
        }
    }

    pub(crate) fn set_applies_to_size(&mut self, size: ManualPrintSize, value: bool) {
        match size {
            ManualPrintSize::A0 => self.applies_a0 = value,
            ManualPrintSize::A1 => self.applies_a1 = value,
            ManualPrintSize::A2 => self.applies_a2 = value,
            ManualPrintSize::A3 => self.applies_a3 = value,
            ManualPrintSize::A4 => self.applies_a4 = value,
        }
    }

    pub(crate) fn price_input(&self, size: ManualPrintSize) -> &str {
        match size {
            ManualPrintSize::A0 => &self.a0_price_input,
            ManualPrintSize::A1 => &self.a1_price_input,
            ManualPrintSize::A2 => &self.a2_price_input,
            ManualPrintSize::A3 => &self.a3_price_input,
            ManualPrintSize::A4 => &self.a4_price_input,
        }
    }

    pub(crate) fn set_price_input(&mut self, size: ManualPrintSize, value: String) {
        match size {
            ManualPrintSize::A0 => self.a0_price_input = value,
            ManualPrintSize::A1 => self.a1_price_input = value,
            ManualPrintSize::A2 => self.a2_price_input = value,
            ManualPrintSize::A3 => self.a3_price_input = value,
            ManualPrintSize::A4 => self.a4_price_input = value,
        }
    }

    pub(crate) fn display_name(&self) -> String {
        let trimmed = self.name_input.trim();
        if trimmed.is_empty() {
            "Unnamed modifier".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

impl Default for ManualPaperModifier {
    fn default() -> Self {
        Self {
            name_input: "300G".to_string(),
            legacy_price_input: String::new(),
            applies_a0: true,
            a0_price_input: "1.00".to_string(),
            applies_a1: true,
            a1_price_input: "1.00".to_string(),
            applies_a2: true,
            a2_price_input: "1.00".to_string(),
            applies_a3: true,
            a3_price_input: "1.00".to_string(),
            applies_a4: true,
            a4_price_input: "1.00".to_string(),
        }
    }
}

fn default_manual_paper_modifiers() -> Vec<ManualPaperModifier> {
    vec![ManualPaperModifier::default()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualModifierChoice {
    pub(crate) index: Option<usize>,
    pub(crate) label: String,
}

impl std::fmt::Display for ManualModifierChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualPricingSettings {
    pub(crate) a0_input: String,
    pub(crate) a1_input: String,
    pub(crate) a2_input: String,
    pub(crate) a3_input: String,
    pub(crate) a4_input: String,
    #[serde(default)]
    pub(crate) a3_bw_first_input: String,
    #[serde(default)]
    pub(crate) a3_bw_next_input: String,
    #[serde(default)]
    pub(crate) a3_bw_rest_input: String,
    #[serde(default)]
    pub(crate) a3_color_first_input: String,
    #[serde(default)]
    pub(crate) a3_color_rest_input: String,
    #[serde(default)]
    pub(crate) a4_bw_first_input: String,
    #[serde(default)]
    pub(crate) a4_bw_next_input: String,
    #[serde(default)]
    pub(crate) a4_bw_rest_input: String,
    #[serde(default)]
    pub(crate) a4_color_first_input: String,
    #[serde(default)]
    pub(crate) a4_color_rest_input: String,
    #[serde(default)]
    pub(crate) laminate_a0_input: String,
    #[serde(default)]
    pub(crate) laminate_a1_input: String,
    #[serde(default)]
    pub(crate) laminate_a2_input: String,
    #[serde(default)]
    pub(crate) laminate_a3_input: String,
    #[serde(default)]
    pub(crate) laminate_a4_input: String,
    #[serde(default)]
    pub(crate) laminate_a5_input: String,
    #[serde(default)]
    pub(crate) folding_input: String,
    #[serde(default)]
    pub(crate) binding_input: String,
    #[serde(default = "default_manual_paper_modifiers")]
    pub(crate) modifiers: Vec<ManualPaperModifier>,
    #[serde(default)]
    pub(crate) line_items: Vec<ManualPricingLineItem>,
    #[serde(default)]
    pub(crate) finisher_items: Vec<ManualFinisherLineItem>,
    #[serde(default)]
    pub(crate) cutting_enabled: bool,
    #[serde(default)]
    pub(crate) discount_input: String,
    #[serde(default)]
    pub(crate) rounding_mode: ManualRoundingMode,
}

impl ManualPricingSettings {
    pub(crate) fn size_price_input(&self, size: ManualPrintSize) -> &str {
        match size {
            ManualPrintSize::A0 => &self.a0_input,
            ManualPrintSize::A1 => &self.a1_input,
            ManualPrintSize::A2 => &self.a2_input,
            ManualPrintSize::A3 => &self.a3_input,
            ManualPrintSize::A4 => &self.a4_input,
        }
    }

    pub(crate) fn set_size_price_input(&mut self, size: ManualPrintSize, value: String) {
        match size {
            ManualPrintSize::A0 => self.a0_input = value,
            ManualPrintSize::A1 => self.a1_input = value,
            ManualPrintSize::A2 => self.a2_input = value,
            ManualPrintSize::A3 => self.a3_input = value,
            ManualPrintSize::A4 => self.a4_input = value,
        }
    }

    pub(crate) fn bw_tier_input(&self, size: ManualPrintSize, tier: ManualBwTier) -> Option<&str> {
        match (size, tier) {
            (ManualPrintSize::A3, ManualBwTier::FirstFive) => Some(&self.a3_bw_first_input),
            (ManualPrintSize::A3, ManualBwTier::NextFive) => Some(&self.a3_bw_next_input),
            (ManualPrintSize::A3, ManualBwTier::Rest) => Some(&self.a3_bw_rest_input),
            (ManualPrintSize::A4, ManualBwTier::FirstFive) => Some(&self.a4_bw_first_input),
            (ManualPrintSize::A4, ManualBwTier::NextFive) => Some(&self.a4_bw_next_input),
            (ManualPrintSize::A4, ManualBwTier::Rest) => Some(&self.a4_bw_rest_input),
            _ => None,
        }
    }

    pub(crate) fn set_bw_tier_input(
        &mut self,
        size: ManualPrintSize,
        tier: ManualBwTier,
        value: String,
    ) {
        match (size, tier) {
            (ManualPrintSize::A3, ManualBwTier::FirstFive) => self.a3_bw_first_input = value,
            (ManualPrintSize::A3, ManualBwTier::NextFive) => self.a3_bw_next_input = value,
            (ManualPrintSize::A3, ManualBwTier::Rest) => self.a3_bw_rest_input = value,
            (ManualPrintSize::A4, ManualBwTier::FirstFive) => self.a4_bw_first_input = value,
            (ManualPrintSize::A4, ManualBwTier::NextFive) => self.a4_bw_next_input = value,
            (ManualPrintSize::A4, ManualBwTier::Rest) => self.a4_bw_rest_input = value,
            _ => {}
        }
    }

    pub(crate) fn color_tier_input(
        &self,
        size: ManualPrintSize,
        tier: ManualColorTier,
    ) -> Option<&str> {
        match (size, tier) {
            (ManualPrintSize::A3, ManualColorTier::FirstFive) => Some(&self.a3_color_first_input),
            (ManualPrintSize::A3, ManualColorTier::Rest) => Some(&self.a3_color_rest_input),
            (ManualPrintSize::A4, ManualColorTier::FirstFive) => Some(&self.a4_color_first_input),
            (ManualPrintSize::A4, ManualColorTier::Rest) => Some(&self.a4_color_rest_input),
            _ => None,
        }
    }

    pub(crate) fn set_color_tier_input(
        &mut self,
        size: ManualPrintSize,
        tier: ManualColorTier,
        value: String,
    ) {
        match (size, tier) {
            (ManualPrintSize::A3, ManualColorTier::FirstFive) => self.a3_color_first_input = value,
            (ManualPrintSize::A3, ManualColorTier::Rest) => self.a3_color_rest_input = value,
            (ManualPrintSize::A4, ManualColorTier::FirstFive) => self.a4_color_first_input = value,
            (ManualPrintSize::A4, ManualColorTier::Rest) => self.a4_color_rest_input = value,
            _ => {}
        }
    }

    pub(crate) fn laminate_price_input(&self, size: ManualLaminateSize) -> &str {
        match size {
            ManualLaminateSize::A0 => &self.laminate_a0_input,
            ManualLaminateSize::A1 => &self.laminate_a1_input,
            ManualLaminateSize::A2 => &self.laminate_a2_input,
            ManualLaminateSize::A3 => &self.laminate_a3_input,
            ManualLaminateSize::A4 => &self.laminate_a4_input,
            ManualLaminateSize::A5 => &self.laminate_a5_input,
        }
    }

    pub(crate) fn set_laminate_price_input(&mut self, size: ManualLaminateSize, value: String) {
        match size {
            ManualLaminateSize::A0 => self.laminate_a0_input = value,
            ManualLaminateSize::A1 => self.laminate_a1_input = value,
            ManualLaminateSize::A2 => self.laminate_a2_input = value,
            ManualLaminateSize::A3 => self.laminate_a3_input = value,
            ManualLaminateSize::A4 => self.laminate_a4_input = value,
            ManualLaminateSize::A5 => self.laminate_a5_input = value,
        }
    }

    pub(crate) fn reset_calculator_state(&mut self) {
        self.line_items = vec![ManualPricingLineItem::default()];
        self.finisher_items.clear();
        self.cutting_enabled = false;
        self.discount_input.clear();
        self.rounding_mode = ManualRoundingMode::FiveCents;
    }

    pub(crate) fn normalize(&mut self) {
        if self.modifiers.is_empty() {
            self.modifiers = default_manual_paper_modifiers();
        }
        if self.line_items.is_empty() {
            self.line_items = vec![ManualPricingLineItem::default()];
        }

        for modifier in &mut self.modifiers {
            if !modifier.legacy_price_input.trim().is_empty() {
                for size in ManualPrintSize::ALL {
                    if modifier.applies_to_size(size)
                        && modifier.price_input(size).trim().is_empty()
                    {
                        modifier.set_price_input(size, modifier.legacy_price_input.clone());
                    }
                }
            }
        }

        for size in [ManualPrintSize::A3, ManualPrintSize::A4] {
            let fallback = self.size_price_input(size).to_string();
            for tier in [
                ManualBwTier::FirstFive,
                ManualBwTier::NextFive,
                ManualBwTier::Rest,
            ] {
                if self
                    .bw_tier_input(size, tier)
                    .is_some_and(|value| value.trim().is_empty())
                {
                    self.set_bw_tier_input(size, tier, fallback.clone());
                }
            }
            for tier in [ManualColorTier::FirstFive, ManualColorTier::Rest] {
                if self
                    .color_tier_input(size, tier)
                    .is_some_and(|value| value.trim().is_empty())
                {
                    self.set_color_tier_input(size, tier, fallback.clone());
                }
            }
        }

        for line_item in &mut self.line_items {
            if line_item
                .modifier_index
                .is_some_and(|index| index >= self.modifiers.len())
            {
                line_item.modifier_index = None;
            }
            line_item.sync_sheets_from_sides();
        }
    }
}

impl Default for ManualPricingSettings {
    fn default() -> Self {
        Self {
            a0_input: "0.00".to_string(),
            a1_input: "0.00".to_string(),
            a2_input: "0.00".to_string(),
            a3_input: "1.00".to_string(),
            a4_input: "0.00".to_string(),
            a3_bw_first_input: "1.00".to_string(),
            a3_bw_next_input: "1.00".to_string(),
            a3_bw_rest_input: "1.00".to_string(),
            a3_color_first_input: "1.00".to_string(),
            a3_color_rest_input: "1.00".to_string(),
            a4_bw_first_input: "0.00".to_string(),
            a4_bw_next_input: "0.00".to_string(),
            a4_bw_rest_input: "0.00".to_string(),
            a4_color_first_input: "0.00".to_string(),
            a4_color_rest_input: "0.00".to_string(),
            laminate_a0_input: "0.00".to_string(),
            laminate_a1_input: "0.00".to_string(),
            laminate_a2_input: "0.00".to_string(),
            laminate_a3_input: "0.00".to_string(),
            laminate_a4_input: "0.00".to_string(),
            laminate_a5_input: "0.00".to_string(),
            folding_input: "0.00".to_string(),
            binding_input: "0.00".to_string(),
            modifiers: default_manual_paper_modifiers(),
            line_items: vec![ManualPricingLineItem::default()],
            finisher_items: Vec::new(),
            cutting_enabled: false,
            discount_input: String::new(),
            rounding_mode: ManualRoundingMode::FiveCents,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPricingBill {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) subject: String,
    #[serde(default)]
    pub(crate) pricing: ManualPricingSettings,
}

impl Default for ManualPricingBill {
    fn default() -> Self {
        Self {
            id: String::new(),
            subject: String::new(),
            pricing: ManualPricingSettings::default(),
        }
    }
}

impl ManualPricingBill {
    pub(crate) fn display_subject(&self) -> &str {
        let subject = self.subject.trim();
        if subject.is_empty() {
            self.id.as_str()
        } else {
            subject
        }
    }

    pub(crate) fn normalize(&mut self) {
        self.pricing.normalize();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManualPricingWorkspace {
    #[serde(default)]
    pub(crate) settings: ManualPricingSettings,
    #[serde(default)]
    pub(crate) bills: Vec<ManualPricingBill>,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    LogTick,
    SyncTick,
    SyncEvent(SyncEvent),
    ToggleAdvancedMode,
    DragWindow,
    MinimizeWindow,
    CloseWindow,
    LogLevelChanged(LogLevel),
    ToggleTarget(String, bool),
    CopyDiagnostics,
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
    SelectManualPricing,
    SelectManualPricingBill(String),
    SelectManualPricingTab(ManualPricingTab),
    SelectPrinterTab(PrinterTab),
    SelectPrinter(PrinterId),
    ProfileChoiceChanged(ProfileChoice),
    DeleteSelectedPrinter,
    PollSelectedSnmp,
    PollPrinterById(PrinterId),
    PollExportPathChanged(String),
    ExportPollData,
    SnmpPolled {
        printer_id: PrinterId,
        result: Result<SnmpResponse, SnmpErrorInfo>,
    },
    OidsPathChanged(String),
    OidsTotalChanged(String),
    ApplyOids,
    LoadOids,
    SaveOids,
    CrawlOids,
    OidsCrawled(Result<CounterOidSet, SnmpErrorInfo>),
    RecordingOidCopiesBwChanged(String),
    RecordingOidCopiesColorChanged(String),
    RecordingOidPrintsBwChanged(String),
    RecordingOidPrintsColorChanged(String),
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
    RecordingEndResetToPolled(RecordingCategory),
    RecordingToggleInclude(RecordingCategory),
    RecordingEndFieldsUnlockedChanged(bool),
    PricingBwFirstChanged(String),
    PricingBwNextChanged(String),
    PricingBwRestChanged(String),
    PricingColorChanged(String),
    PricingRoundChanged(bool),
    SaveManualPricingAsBill,
    DeleteSelectedManualPricingBill,
    ManualPricingBillSubjectChanged(String),
    ManualPricingLineAdded,
    ManualPricingLineRemoved(usize),
    ManualPricingLineSizeChanged(usize, ManualPrintSize),
    ManualPricingLinePrintModeChanged(usize, ManualPrintMode),
    ManualPricingLineModifierChanged(usize, Option<usize>),
    ManualPricingLineSidesChanged(usize, String),
    ManualPricingLineDoubleSidedChanged(usize, bool),
    ManualPricingFinisherAdded,
    ManualPricingFinisherRemoved(usize),
    ManualPricingFinisherTypeChanged(usize, ManualFinisherType),
    ManualPricingFinisherSizeChanged(usize, ManualLaminateSize),
    ManualPricingFinisherAmountChanged(usize, String),
    ManualPricingBasePriceChanged(ManualPrintSize, String),
    ManualPricingBwTierChanged(ManualPrintSize, ManualBwTier, String),
    ManualPricingColorTierChanged(ManualPrintSize, ManualColorTier, String),
    ManualPricingLaminatePriceChanged(ManualLaminateSize, String),
    ManualPricingFoldingPriceChanged(String),
    ManualPricingBindingPriceChanged(String),
    ManualPricingModifierAdded,
    ManualPricingModifierRemoved(usize),
    ManualPricingModifierNameChanged(usize, String),
    ManualPricingModifierPriceChanged(usize, ManualPrintSize, String),
    ManualPricingModifierAppliesChanged(usize, ManualPrintSize, bool),
    ManualPricingCuttingChanged(bool),
    ManualPricingDiscountChanged(String),
    ManualPricingRoundingToggled(ManualRoundingMode, bool),
    ManualPricingPathChanged(String),
    LoadManualPricing,
    SaveManualPricing,
    SyncPrices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpErrorInfo {
    pub(crate) status: PrinterStatus,
    pub(crate) summary: String,
    pub(crate) detail: String,
}

impl SnmpErrorInfo {
    pub(crate) fn new(
        status: PrinterStatus,
        summary: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status,
            summary: summary.into(),
            detail: detail.into(),
        }
    }

    pub(crate) fn from_error(error: CoreError) -> Self {
        let status = match &error {
            CoreError::SnmpTimeout { .. } => PrinterStatus::Offline,
            CoreError::SnmpFailure { details, .. } if is_reachability_error(details) => {
                PrinterStatus::Offline
            }
            CoreError::SnmpAuth { .. } | CoreError::SnmpFailure { .. } => PrinterStatus::Error,
            _ => PrinterStatus::Error,
        };

        Self::new(status, error.user_summary(), error.technical_detail())
    }
}

fn is_reachability_error(details: &str) -> bool {
    let detail = details.to_ascii_lowercase();
    [
        "timed out",
        "unreachable",
        "no route to host",
        "host is down",
        "network is unreachable",
        "network name is no longer available",
    ]
    .iter()
    .any(|needle| detail.contains(needle))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnmpPollStatus {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingSession {
    pub(crate) active: bool,
    pub(crate) start: Option<RecordingSnapshot>,
    pub(crate) end: Option<RecordingSnapshot>,
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) end_fields_unlocked: bool,
    pub(crate) edits: RecordingEdits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecordingOidSettings {
    pub(crate) copies_bw_input: String,
    pub(crate) copies_color_input: String,
    pub(crate) prints_bw_input: String,
    pub(crate) prints_color_input: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingSettings {
    pub(crate) bw_first_input: String,
    pub(crate) bw_next_input: String,
    pub(crate) bw_rest_input: String,
    pub(crate) color_input: String,
    #[serde(rename = "round_to_half_euro", alias = "round_to_five_cents")]
    pub(crate) round_to_five_cents: bool,
    #[serde(default)]
    pub(crate) manual_pricing: ManualPricingSettings,
}

impl Default for PricingSettings {
    fn default() -> Self {
        Self {
            bw_first_input: "0.25".to_string(),
            bw_next_input: "0.10".to_string(),
            bw_rest_input: "0.06".to_string(),
            color_input: "0.50".to_string(),
            round_to_five_cents: true,
            manual_pricing: ManualPricingSettings::default(),
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

#[derive(Clone)]
pub struct Flags {
    pub log_store: LogStore,
    pub reload_handle: ReloadHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileChoice {
    Auto,
    Profile(String),
}

impl std::fmt::Display for ProfileChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileChoice::Auto => write!(f, "Auto match"),
            ProfileChoice::Profile(id) => write!(f, "{id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_pricing_line_item_defaults_to_color_a3_with_zero_sides() {
        let default_line = ManualPricingLineItem::default();

        assert_eq!(default_line.size, ManualPrintSize::A3);
        assert_eq!(default_line.print_mode, ManualPrintMode::Color);
        assert_eq!(default_line.modifier_index, None);
        assert_eq!(default_line.sides_input, "0");
        assert_eq!(default_line.sheets_input, "0");
    }

    #[test]
    fn manual_pricing_settings_default_to_five_cent_rounding() {
        let default_settings = ManualPricingSettings::default();

        assert_eq!(default_settings.rounding_mode, ManualRoundingMode::FiveCents);
        assert!(default_settings.finisher_items.is_empty());
        assert_eq!(
            default_settings.line_items,
            vec![ManualPricingLineItem::default()]
        );
    }

    #[test]
    fn manual_pricing_reset_calculator_state_preserves_price_configuration() {
        let modifiers = vec![ManualPaperModifier {
            name_input: "Satin".to_string(),
            legacy_price_input: String::new(),
            applies_a0: false,
            a0_price_input: "0.00".to_string(),
            applies_a1: false,
            a1_price_input: "0.00".to_string(),
            applies_a2: false,
            a2_price_input: "0.00".to_string(),
            applies_a3: true,
            a3_price_input: "2.25".to_string(),
            applies_a4: true,
            a4_price_input: "1.25".to_string(),
        }];
        let mut settings = ManualPricingSettings {
            a0_input: "30.00".to_string(),
            a1_input: "20.00".to_string(),
            a2_input: "10.00".to_string(),
            a3_input: "3.25".to_string(),
            a4_input: "1.50".to_string(),
            binding_input: "7.50".to_string(),
            modifiers: modifiers.clone(),
            line_items: vec![ManualPricingLineItem {
                size: ManualPrintSize::A0,
                print_mode: ManualPrintMode::Bw,
                modifier_index: Some(0),
                double_sided: false,
                sheets_input: "4".to_string(),
                sides_input: "4".to_string(),
            }],
            finisher_items: vec![ManualFinisherLineItem {
                finisher_type: ManualFinisherType::Binding,
                laminate_size: ManualLaminateSize::A4,
                amount_input: "3".to_string(),
            }],
            cutting_enabled: true,
            discount_input: "12".to_string(),
            rounding_mode: ManualRoundingMode::HalfEuro,
            ..ManualPricingSettings::default()
        };

        settings.reset_calculator_state();

        assert_eq!(settings.a0_input, "30.00");
        assert_eq!(settings.a3_input, "3.25");
        assert_eq!(settings.binding_input, "7.50");
        assert_eq!(settings.modifiers, modifiers);
        assert_eq!(settings.line_items, vec![ManualPricingLineItem::default()]);
        assert!(settings.finisher_items.is_empty());
        assert!(!settings.cutting_enabled);
        assert!(settings.discount_input.is_empty());
        assert_eq!(settings.rounding_mode, ManualRoundingMode::FiveCents);
    }
}
