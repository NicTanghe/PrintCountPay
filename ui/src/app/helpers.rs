use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use iced::Color;
use iced::keyboard;
use printcountpay_core::{CounterOidSet, Oid, PrinterStatus, SnmpVarBind};
use time::{Date, Month, OffsetDateTime, UtcOffset};

use crate::app::constants::{
    PRT_GENERAL_PRINTER_NAME_OID, PRT_MARKER_LIFECOUNT_1, PRT_MARKER_LIFECOUNT_2,
    PRT_MARKER_LIFECOUNT_3, RICOH_BW_COPIER_COUNT_OID, RICOH_BW_PRINTER_COUNT_OID,
    RICOH_COLOR_COPIER_COUNT_OID, RICOH_COLOR_PRINTER_COUNT_OID, RICOH_COUNTER_TABLE,
    RICOH_COUNTER_VALUE_ROOT, RICOH_TONER_BLACK_OID, RICOH_TONER_CYAN_OID, RICOH_TONER_MAGENTA_OID,
    RICOH_TONER_YELLOW_OID, SYS_DESCR_OID, SYS_NAME_OID, SYS_OBJECT_ID_OID, SYS_UPTIME_OID,
};
use crate::app::profiles::{ManufacturerProfile, RecordingOidProfile, TonerOidProfile};
use crate::app::types::{
    BwPricing, ManualBwTier, ManualColorTier, ManualFinisherLineItem, ManualFinisherType,
    ManualPricingLineItem, ManualPricingSettings, ManualPrintMode, ManualPrintSize,
    ManualRoundingMode, Message, PricingSettings, RecordingCategory, RecordingOidSettings,
    RecordingSession, RecordingSnapshot, SnmpPollStatus,
};

pub(crate) fn level_color(level: tracing::Level) -> Color {
    match level {
        tracing::Level::ERROR => Color::from_rgb8(0xe0, 0x4f, 0x4f),
        tracing::Level::WARN => Color::from_rgb8(0xe0, 0xb0, 0x4f),
        tracing::Level::INFO => Color::from_rgb8(0x3b, 0x82, 0xf6),
        tracing::Level::DEBUG => Color::from_rgb8(0x22, 0x7d, 0x64),
        tracing::Level::TRACE => Color::from_rgb8(0x6b, 0x72, 0x80),
    }
}

pub(crate) fn delete_key_event(
    key: keyboard::Key,
    _modifiers: keyboard::Modifiers,
) -> Option<Message> {
    match key {
        keyboard::Key::Named(keyboard::key::Named::Delete) => Some(Message::DeleteSelectedPrinter),
        _ => None,
    }
}

pub(crate) fn status_label(status: PrinterStatus) -> &'static str {
    match status {
        PrinterStatus::Unknown => "Unknown",
        PrinterStatus::Online => "Online",
        PrinterStatus::Offline => "Offline",
        PrinterStatus::Error => "Error",
    }
}

pub(crate) fn recording_category_label(category: RecordingCategory) -> &'static str {
    match category {
        RecordingCategory::CopiesBw => "Copies B/W",
        RecordingCategory::CopiesColor => "Copies color",
        RecordingCategory::PrintsBw => "Prints B/W",
        RecordingCategory::PrintsColor => "Prints color",
    }
}

pub(crate) fn missing_recording_snapshot_categories(
    snapshot: &RecordingSnapshot,
    recording_oids: &RecordingOidProfile,
) -> Vec<RecordingCategory> {
    let mut missing = Vec::new();

    if !recording_oids.copies_bw.is_empty() && snapshot.bw_copier.is_none() {
        missing.push(RecordingCategory::CopiesBw);
    }
    if !recording_oids.copies_color.is_empty() && snapshot.color_copier.is_none() {
        missing.push(RecordingCategory::CopiesColor);
    }
    if !recording_oids.prints_bw.is_empty() && snapshot.bw_printer.is_none() {
        missing.push(RecordingCategory::PrintsBw);
    }
    if !recording_oids.prints_color.is_empty() && snapshot.color_printer.is_none() {
        missing.push(RecordingCategory::PrintsColor);
    }

    missing
}

pub(crate) fn format_recording_category_list(categories: &[RecordingCategory]) -> String {
    categories
        .iter()
        .map(|category| recording_category_label(*category).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn printer_card_tint(index: usize, total: usize) -> Color {
    let start = (231.0, 0.13, 0.89);
    let end = (232.0, 0.12, 0.87);
    let t = if total <= 1 {
        0.0
    } else {
        index as f32 / (total.saturating_sub(1)) as f32
    };

    hsl_to_rgb(
        lerp(start.0, end.0, t),
        lerp(start.1, end.1, t),
        lerp(start.2, end.2, t),
    )
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Color {
    let h = (h.rem_euclid(360.0)) / 360.0;

    if s <= f32::EPSILON {
        return Color::from_rgb(l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    Color::from_rgb(
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }

    p
}

pub(crate) fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub(crate) fn format_elapsed_hms(total_seconds: u64) -> String {
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub(crate) fn format_clock_hms(epoch_seconds: u64) -> String {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    format_clock_hms_with_offset(epoch_seconds, offset)
}

pub(crate) fn format_calendar_date(date: Date) -> String {
    format!(
        "{:02} {} {}",
        date.day(),
        short_month_label(date.month()),
        date.year()
    )
}

pub(crate) fn format_local_date_time(epoch_seconds: u64) -> String {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    format_local_date_time_with_offset(epoch_seconds, offset)
}

fn format_clock_hms_with_offset(epoch_seconds: u64, offset: UtcOffset) -> String {
    if epoch_seconds > i64::MAX as u64 {
        return "n/a".to_string();
    }

    match OffsetDateTime::from_unix_timestamp(epoch_seconds as i64) {
        Ok(timestamp) => {
            let local = timestamp.to_offset(offset);
            format!(
                "{:02}:{:02}:{:02}",
                local.hour(),
                local.minute(),
                local.second()
            )
        }
        Err(_) => "n/a".to_string(),
    }
}

fn format_local_date_time_with_offset(epoch_seconds: u64, offset: UtcOffset) -> String {
    if epoch_seconds > i64::MAX as u64 {
        return "n/a".to_string();
    }

    match OffsetDateTime::from_unix_timestamp(epoch_seconds as i64) {
        Ok(timestamp) => {
            let local = timestamp.to_offset(offset);
            format!(
                "{:02} {} {} {:02}:{:02}",
                local.day(),
                short_month_label(local.month()),
                local.year(),
                local.hour(),
                local.minute(),
            )
        }
        Err(_) => "n/a".to_string(),
    }
}

fn short_month_label(month: Month) -> &'static str {
    match month {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    }
}

pub(crate) fn poll_received_at(state: &SnmpPollStatus) -> Option<u64> {
    match state {
        SnmpPollStatus::Idle => None,
        SnmpPollStatus::Ok { received_at, .. } | SnmpPollStatus::Error { received_at, .. } => {
            Some(*received_at)
        }
    }
}

pub(crate) fn default_counter_oids() -> CounterOidSet {
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

pub(crate) fn default_recording_oid_inputs() -> RecordingOidSettings {
    let copies_color_alt = ricoh_counter_oid(202);
    let prints_color_alt = ricoh_counter_oid(402);
    RecordingOidSettings {
        copies_bw_input: format_oid_list(&[Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID)]),
        copies_color_input: format_oid_list(&[
            Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID),
            copies_color_alt,
        ]),
        prints_bw_input: format_oid_list(&[Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID)]),
        prints_color_input: format_oid_list(&[
            Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID),
            prints_color_alt,
        ]),
    }
}

pub(crate) fn default_toner_oids() -> TonerOidProfile {
    TonerOidProfile {
        black: Some(Oid::from_slice(&RICOH_TONER_BLACK_OID)),
        cyan: Some(Oid::from_slice(&RICOH_TONER_CYAN_OID)),
        magenta: Some(Oid::from_slice(&RICOH_TONER_MAGENTA_OID)),
        yellow: Some(Oid::from_slice(&RICOH_TONER_YELLOW_OID)),
    }
}

pub(crate) fn recording_oids_from_counter_set(set: &CounterOidSet) -> RecordingOidSettings {
    let mut copies_bw = Vec::new();
    let mut prints_bw = Vec::new();
    let mut copies_color = Vec::new();
    let mut prints_color = Vec::new();
    let copies_color_alt = ricoh_counter_oid(202);
    let prints_color_alt = ricoh_counter_oid(402);

    for oid in &set.bw {
        if oid.as_slice() == RICOH_BW_COPIER_COUNT_OID.as_slice() {
            copies_bw.push(oid.clone());
        } else if oid.as_slice() == RICOH_BW_PRINTER_COUNT_OID.as_slice() {
            prints_bw.push(oid.clone());
        } else {
            copies_bw.push(oid.clone());
        }
    }

    for oid in &set.color {
        if oid.as_slice() == RICOH_COLOR_COPIER_COUNT_OID.as_slice()
            || oid.as_slice() == copies_color_alt.as_slice()
        {
            copies_color.push(oid.clone());
        } else if oid.as_slice() == RICOH_COLOR_PRINTER_COUNT_OID.as_slice()
            || oid.as_slice() == prints_color_alt.as_slice()
        {
            prints_color.push(oid.clone());
        } else {
            copies_color.push(oid.clone());
        }
    }

    RecordingOidSettings {
        copies_bw_input: format_oid_list(&copies_bw),
        copies_color_input: format_oid_list(&copies_color),
        prints_bw_input: format_oid_list(&prints_bw),
        prints_color_input: format_oid_list(&prints_color),
    }
}

pub(crate) fn recording_settings_from_profile(
    profile: &RecordingOidProfile,
) -> RecordingOidSettings {
    RecordingOidSettings {
        copies_bw_input: format_oid_list(&profile.copies_bw),
        copies_color_input: format_oid_list(&profile.copies_color),
        prints_bw_input: format_oid_list(&profile.prints_bw),
        prints_color_input: format_oid_list(&profile.prints_color),
    }
}

pub(crate) fn recording_profile_from_settings(
    settings: &RecordingOidSettings,
) -> Result<RecordingOidProfile, String> {
    Ok(RecordingOidProfile {
        copies_bw: parse_oid_list(&settings.copies_bw_input)
            .map_err(|error| format!("Copies B/W OIDs: {error}"))?,
        copies_color: parse_oid_list(&settings.copies_color_input)
            .map_err(|error| format!("Copies color OIDs: {error}"))?,
        prints_bw: parse_oid_list(&settings.prints_bw_input)
            .map_err(|error| format!("Prints B/W OIDs: {error}"))?,
        prints_color: parse_oid_list(&settings.prints_color_input)
            .map_err(|error| format!("Prints color OIDs: {error}"))?,
    })
}

pub(crate) fn recording_profile_from_settings_lossy(
    settings: &RecordingOidSettings,
) -> RecordingOidProfile {
    RecordingOidProfile {
        copies_bw: parse_oid_list(&settings.copies_bw_input).unwrap_or_default(),
        copies_color: parse_oid_list(&settings.copies_color_input).unwrap_or_default(),
        prints_bw: parse_oid_list(&settings.prints_bw_input).unwrap_or_default(),
        prints_color: parse_oid_list(&settings.prints_color_input).unwrap_or_default(),
    }
}

pub(crate) fn build_poll_label_map(
    counter_oids: &CounterOidSet,
    recording_settings: &RecordingOidSettings,
    profile: Option<&ManufacturerProfile>,
) -> std::collections::HashMap<Oid, String> {
    let mut map = std::collections::HashMap::new();
    let recording_oids = recording_profile_from_settings_lossy(recording_settings);
    let mut insert_label = |oid: Oid, label: &str| {
        map.entry(oid).or_insert_with(|| label.to_string());
    };

    insert_label(Oid::from_slice(&SYS_DESCR_OID), "System: Description");
    insert_label(Oid::from_slice(&SYS_OBJECT_ID_OID), "System: Object ID");
    insert_label(Oid::from_slice(&SYS_NAME_OID), "System: Name");
    insert_label(Oid::from_slice(&SYS_UPTIME_OID), "System: Uptime");
    insert_label(
        Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID),
        "Printer: Name",
    );

    if profile.and_then(|profile| profile.counter_table.as_deref()) == Some("ricoh-m184") {
        for entry in &RICOH_COUNTER_TABLE {
            insert_label(ricoh_counter_oid(entry.type_id), entry.label);
        }
    }

    for oid in &recording_oids.copies_bw {
        insert_label(oid.clone(), "Recording: Copies B/W");
    }
    for oid in &recording_oids.copies_color {
        insert_label(oid.clone(), "Recording: Copies Color");
    }
    for oid in &recording_oids.prints_bw {
        insert_label(oid.clone(), "Recording: Prints B/W");
    }
    for oid in &recording_oids.prints_color {
        insert_label(oid.clone(), "Recording: Prints Color");
    }

    for oid in &counter_oids.bw {
        insert_label(oid.clone(), "Clicks: B/W");
    }
    for oid in &counter_oids.color {
        insert_label(oid.clone(), "Clicks: Color");
    }
    for oid in &counter_oids.total {
        insert_label(oid.clone(), "Clicks: Total");
    }

    let default_toner = default_toner_oids();
    let toner = profile
        .map(|profile| &profile.toner)
        .unwrap_or(&default_toner);
    if let Some(oid) = toner.black.as_ref() {
        insert_label(oid.clone(), "Toner: Black");
    }
    if let Some(oid) = toner.cyan.as_ref() {
        insert_label(oid.clone(), "Toner: Cyan");
    }
    if let Some(oid) = toner.magenta.as_ref() {
        insert_label(oid.clone(), "Toner: Magenta");
    }
    if let Some(oid) = toner.yellow.as_ref() {
        insert_label(oid.clone(), "Toner: Yellow");
    }

    if let Some(profile) = profile {
        for entry in &profile.extra_poll_labels {
            map.entry(entry.oid.clone())
                .or_insert_with(|| entry.label.clone());
        }
    }

    map
}

pub(crate) fn format_oid_list(oids: &[Oid]) -> String {
    oids.iter()
        .map(|oid| oid.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

pub(crate) fn ricoh_counter_oid(type_id: u32) -> Oid {
    let mut parts = Vec::with_capacity(RICOH_COUNTER_VALUE_ROOT.len() + 1);
    parts.extend_from_slice(&RICOH_COUNTER_VALUE_ROOT);
    parts.push(type_id);
    Oid(parts)
}

pub(crate) fn parse_oid_list(value: &str) -> Result<Vec<Oid>, String> {
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

pub(crate) fn delta_value(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    let start = start?;
    let end = end?;
    end.checked_sub(start)
}

pub(crate) fn sum_two(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

pub(crate) fn bw_cost_cents(count: u64, pricing: BwPricing) -> u64 {
    let first = count.min(5);
    let second = count.saturating_sub(5).min(5);
    let rest = count.saturating_sub(10);
    first * pricing.first_cents + second * pricing.next_cents + rest * pricing.rest_cents
}

pub(crate) fn color_cost_cents(count: u64, price_cents: u64) -> u64 {
    count * price_cents
}

pub(crate) fn round_to_nearest_5_cents(total_cents: u64) -> u64 {
    (total_cents + 2) / 5 * 5
}

pub(crate) fn format_cents(cents: u64) -> String {
    let euros = cents / 100;
    let remainder = cents % 100;
    format!("{euros}.{remainder:02} EUR")
}

pub(crate) fn format_count(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".to_string())
}

pub(crate) fn parse_count_input(value: &str) -> Result<Option<u64>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed.parse::<u64>().map(Some).map_err(|_| ())
}

pub(crate) fn parse_price_input(value: &str) -> Result<Option<u64>, ()> {
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

pub(crate) fn parse_percentage_input(value: &str) -> Result<Option<u64>, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.replace(',', ".");
    let parsed = normalized.parse::<f64>().map_err(|_| ())?;
    if !(0.0..=100.0).contains(&parsed) {
        return Err(());
    }
    Ok(Some((parsed * 100.0).round() as u64))
}

pub(crate) fn bw_pricing_from_settings(settings: &PricingSettings) -> Option<BwPricing> {
    let first = parse_price_input(&settings.bw_first_input).ok().flatten()?;
    let next = parse_price_input(&settings.bw_next_input).ok().flatten()?;
    let rest = parse_price_input(&settings.bw_rest_input).ok().flatten()?;
    Some(BwPricing {
        first_cents: first,
        next_cents: next,
        rest_cents: rest,
    })
}

pub(crate) fn color_price_from_settings(settings: &PricingSettings) -> Option<u64> {
    parse_price_input(&settings.color_input).ok().flatten()
}

pub(crate) const MANUAL_CUTTING_CENTS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManualLineBreakdown {
    pub(crate) sheets: u64,
    pub(crate) sides: u64,
    pub(crate) print_pricing_label: String,
    pub(crate) paper_price_cents: u64,
    pub(crate) print_total_cents: u64,
    pub(crate) paper_total_cents: u64,
    pub(crate) total_cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManualLineState {
    Empty,
    Invalid,
    Ready(ManualLineBreakdown),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ManualPrintCounters {
    bw_tiered_sides: u64,
    color_tiered_sides: u64,
}

impl ManualPrintCounters {
    fn tiered_sides_for(self, mode: ManualPrintMode) -> u64 {
        match mode {
            ManualPrintMode::Bw => self.bw_tiered_sides,
            ManualPrintMode::Color => self.color_tiered_sides,
        }
    }

    fn add_tiered_sides(&mut self, mode: Option<ManualPrintMode>, count: u64) {
        match mode {
            Some(ManualPrintMode::Bw) => {
                self.bw_tiered_sides = self.bw_tiered_sides.saturating_add(count);
            }
            Some(ManualPrintMode::Color) => {
                self.color_tiered_sides = self.color_tiered_sides.saturating_add(count);
            }
            None => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManualPrintPricing {
    total_cents: u64,
    label: String,
    counter_mode: Option<ManualPrintMode>,
    counter_increment: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManualFinisherBreakdown {
    pub(crate) amount: u64,
    pub(crate) label: String,
    pub(crate) unit_price_cents: u64,
    pub(crate) total_cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManualFinisherState {
    Empty,
    Invalid,
    Ready(ManualFinisherBreakdown),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManualPricingTotals {
    pub(crate) line_states: Vec<ManualLineState>,
    pub(crate) finisher_states: Vec<ManualFinisherState>,
    pub(crate) lines_total_cents: Option<u64>,
    pub(crate) finishers_total_cents: Option<u64>,
    pub(crate) subtotal_cents: Option<u64>,
    pub(crate) cutting_cents: u64,
    pub(crate) discount_basis_points: Option<u64>,
    pub(crate) discount_cents: Option<u64>,
    pub(crate) total_before_rounding_cents: Option<u64>,
    pub(crate) total_cents: Option<u64>,
}

pub(crate) fn manual_round_total_cents(total_cents: u64, mode: ManualRoundingMode) -> u64 {
    let step = match mode {
        ManualRoundingMode::None => return total_cents,
        ManualRoundingMode::FiveCents => 5,
        ManualRoundingMode::HalfEuro => 50,
        ManualRoundingMode::DownToFiveEuro => 500,
        ManualRoundingMode::DownToTenEuro => 1_000,
    };

    total_cents / step * step
}

fn tiered_total_cents(
    count: u64,
    already_counted: u64,
    first_cents: u64,
    next_cents: Option<u64>,
    rest_cents: u64,
) -> u64 {
    fn overlap_count(start: u64, count: u64, tier_start: u64, tier_end: Option<u64>) -> u64 {
        let end = start.saturating_add(count);
        let overlap_start = start.max(tier_start);
        let overlap_end = tier_end.map_or(end, |tier_end| end.min(tier_end));

        overlap_end.saturating_sub(overlap_start)
    }

    let first_count = overlap_count(already_counted, count, 0, Some(5));
    let next_count = next_cents
        .map(|_| overlap_count(already_counted, count, 5, Some(10)))
        .unwrap_or(0);
    let rest_count = overlap_count(
        already_counted,
        count,
        if next_cents.is_some() { 10 } else { 5 },
        None,
    );

    first_count.saturating_mul(first_cents)
        + next_count.saturating_mul(next_cents.unwrap_or(0))
        + rest_count.saturating_mul(rest_cents)
}

fn manual_print_pricing(
    settings: &ManualPricingSettings,
    line_item: &ManualPricingLineItem,
    sides: u64,
    counters: ManualPrintCounters,
) -> Option<ManualPrintPricing> {
    match line_item.size {
        ManualPrintSize::A3 | ManualPrintSize::A4 => match line_item.print_mode {
            ManualPrintMode::Bw => {
                let first = parse_price_input(
                    settings.bw_tier_input(line_item.size, ManualBwTier::FirstFive)?,
                )
                .ok()
                .flatten()?;
                let next = parse_price_input(
                    settings.bw_tier_input(line_item.size, ManualBwTier::NextFive)?,
                )
                .ok()
                .flatten()?;
                let rest =
                    parse_price_input(settings.bw_tier_input(line_item.size, ManualBwTier::Rest)?)
                        .ok()
                        .flatten()?;

                Some(ManualPrintPricing {
                    total_cents: tiered_total_cents(
                        sides,
                        counters.tiered_sides_for(ManualPrintMode::Bw),
                        first,
                        Some(next),
                        rest,
                    ),
                    label: format!(
                        "B/W tiers: 1-5 {}, 6-10 {}, 11+ {}",
                        format_cents(first),
                        format_cents(next),
                        format_cents(rest)
                    ),
                    counter_mode: Some(ManualPrintMode::Bw),
                    counter_increment: sides,
                })
            }
            ManualPrintMode::Color => {
                let first = parse_price_input(
                    settings.color_tier_input(line_item.size, ManualColorTier::FirstFive)?,
                )
                .ok()
                .flatten()?;
                let rest = parse_price_input(
                    settings.color_tier_input(line_item.size, ManualColorTier::Rest)?,
                )
                .ok()
                .flatten()?;

                Some(ManualPrintPricing {
                    total_cents: tiered_total_cents(
                        sides,
                        counters.tiered_sides_for(ManualPrintMode::Color),
                        first,
                        None,
                        rest,
                    ),
                    label: format!(
                        "Color tiers: 1-5 {}, 6+ {}",
                        format_cents(first),
                        format_cents(rest)
                    ),
                    counter_mode: Some(ManualPrintMode::Color),
                    counter_increment: sides,
                })
            }
        },
        _ => {
            let price_cents = parse_price_input(settings.size_price_input(line_item.size))
                .ok()
                .flatten()?;
            Some(ManualPrintPricing {
                total_cents: sides.saturating_mul(price_cents),
                label: format!(
                    "Flat {} {}",
                    line_item.print_mode,
                    format_cents(price_cents)
                ),
                counter_mode: None,
                counter_increment: 0,
            })
        }
    }
}

fn manual_line_state_with_counters(
    settings: &ManualPricingSettings,
    line_item: &ManualPricingLineItem,
    counters: &mut ManualPrintCounters,
) -> ManualLineState {
    let sheets_trimmed = line_item.sheets_input.trim();
    let sides_trimmed = line_item.sides_input.trim();
    if sheets_trimmed.is_empty() && sides_trimmed.is_empty() {
        return ManualLineState::Empty;
    }

    let Some(sheets) = line_item.derived_sheets() else {
        return ManualLineState::Invalid;
    };
    let Some(sides) = parse_count_input(&line_item.sides_input).ok().flatten() else {
        return ManualLineState::Invalid;
    };
    let Some(print_pricing) = manual_print_pricing(settings, line_item, sides, *counters) else {
        return ManualLineState::Invalid;
    };

    let paper_price_cents = match line_item.modifier_index {
        Some(modifier_index) => {
            let Some(modifier) = settings.modifiers.get(modifier_index) else {
                return ManualLineState::Invalid;
            };
            if !modifier.applies_to_size(line_item.size) {
                return ManualLineState::Invalid;
            }
            let Some(price_cents) = parse_price_input(modifier.price_input(line_item.size))
                .ok()
                .flatten()
            else {
                return ManualLineState::Invalid;
            };
            price_cents
        }
        None => 0,
    };

    let paper_total_cents = sheets.saturating_mul(paper_price_cents);
    counters.add_tiered_sides(print_pricing.counter_mode, print_pricing.counter_increment);

    ManualLineState::Ready(ManualLineBreakdown {
        sheets,
        sides,
        print_pricing_label: print_pricing.label,
        paper_price_cents,
        print_total_cents: print_pricing.total_cents,
        paper_total_cents,
        total_cents: print_pricing.total_cents.saturating_add(paper_total_cents),
    })
}

pub(crate) fn manual_finisher_state(
    settings: &ManualPricingSettings,
    finisher_item: &ManualFinisherLineItem,
) -> ManualFinisherState {
    let amount_trimmed = finisher_item.amount_input.trim();
    if amount_trimmed.is_empty() {
        return ManualFinisherState::Empty;
    }

    let Some(amount) = parse_count_input(&finisher_item.amount_input)
        .ok()
        .flatten()
    else {
        return ManualFinisherState::Invalid;
    };

    let (unit_price_cents, label) = match finisher_item.finisher_type {
        ManualFinisherType::Laminate => {
            let Some(price_cents) =
                parse_price_input(settings.laminate_price_input(finisher_item.laminate_size))
                    .ok()
                    .flatten()
            else {
                return ManualFinisherState::Invalid;
            };

            (
                price_cents,
                format!(
                    "Laminate {} @ {}",
                    finisher_item.laminate_size,
                    format_cents(price_cents)
                ),
            )
        }
        ManualFinisherType::Folding => {
            let Some(price_cents) = parse_price_input(&settings.folding_input).ok().flatten()
            else {
                return ManualFinisherState::Invalid;
            };

            (
                price_cents,
                format!("Folding @ {}", format_cents(price_cents)),
            )
        }
        ManualFinisherType::Binding => {
            let Some(price_cents) = parse_price_input(&settings.binding_input).ok().flatten()
            else {
                return ManualFinisherState::Invalid;
            };

            (
                price_cents,
                format!("Binding @ {}", format_cents(price_cents)),
            )
        }
    };

    ManualFinisherState::Ready(ManualFinisherBreakdown {
        amount,
        label,
        unit_price_cents,
        total_cents: amount.saturating_mul(unit_price_cents),
    })
}

pub(crate) fn manual_pricing_totals(settings: &ManualPricingSettings) -> ManualPricingTotals {
    let mut print_counters = ManualPrintCounters::default();
    let line_states: Vec<_> = settings
        .line_items
        .iter()
        .map(|line_item| manual_line_state_with_counters(settings, line_item, &mut print_counters))
        .collect();
    let finisher_states: Vec<_> = settings
        .finisher_items
        .iter()
        .map(|finisher_item| manual_finisher_state(settings, finisher_item))
        .collect();

    let mut line_total_cents = 0u64;
    let mut has_invalid_line = false;
    for line_state in &line_states {
        match line_state {
            ManualLineState::Empty => {}
            ManualLineState::Invalid => has_invalid_line = true,
            ManualLineState::Ready(line) => {
                line_total_cents = line_total_cents.saturating_add(line.total_cents);
            }
        }
    }

    let mut finisher_total_cents = 0u64;
    let mut has_invalid_finisher = false;
    for finisher_state in &finisher_states {
        match finisher_state {
            ManualFinisherState::Empty => {}
            ManualFinisherState::Invalid => has_invalid_finisher = true,
            ManualFinisherState::Ready(finisher) => {
                finisher_total_cents = finisher_total_cents.saturating_add(finisher.total_cents);
            }
        }
    }

    let cutting_cents = if settings.cutting_enabled {
        MANUAL_CUTTING_CENTS
    } else {
        0
    };

    let discount_basis_points = match parse_percentage_input(&settings.discount_input) {
        Ok(Some(value)) => Some(value),
        Ok(None) => Some(0),
        Err(()) => None,
    };

    let lines_total_cents = if has_invalid_line {
        None
    } else {
        Some(line_total_cents)
    };

    let finishers_total_cents = if has_invalid_finisher {
        None
    } else {
        Some(finisher_total_cents)
    };

    let subtotal_cents = match (lines_total_cents, finishers_total_cents) {
        (Some(line_total_cents), Some(finisher_total_cents)) => Some(
            line_total_cents
                .saturating_add(finisher_total_cents)
                .saturating_add(cutting_cents),
        ),
        _ => None,
    };

    let discount_cents = match (subtotal_cents, discount_basis_points) {
        (Some(subtotal_cents), Some(discount_basis_points)) => Some(
            (((subtotal_cents as u128 * discount_basis_points as u128) + 5_000) / 10_000) as u64,
        ),
        _ => None,
    };

    let total_before_rounding_cents = match (subtotal_cents, discount_cents) {
        (Some(subtotal_cents), Some(discount_cents)) => {
            Some(subtotal_cents.saturating_sub(discount_cents))
        }
        _ => None,
    };

    let total_cents = total_before_rounding_cents
        .map(|value| manual_round_total_cents(value, settings.rounding_mode));

    ManualPricingTotals {
        line_states,
        finisher_states,
        lines_total_cents,
        finishers_total_cents,
        subtotal_cents,
        cutting_cents,
        discount_basis_points,
        discount_cents,
        total_before_rounding_cents,
        total_cents,
    }
}

pub(crate) fn snapshot_category_value(
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

fn category_start_source_value(
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

fn category_end_polled_value(
    session: &RecordingSession,
    category: RecordingCategory,
    fallback: Option<&RecordingSnapshot>,
) -> Option<u64> {
    session
        .end
        .as_ref()
        .and_then(|snapshot| snapshot_category_value(snapshot, category))
        .or_else(|| fallback.and_then(|snapshot| snapshot_category_value(snapshot, category)))
}

pub(crate) fn category_start_display(
    session: &RecordingSession,
    category: RecordingCategory,
) -> String {
    category_start_source_value(session, category)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

pub(crate) fn category_end_display(
    session: &RecordingSession,
    category: RecordingCategory,
    fallback: Option<&RecordingSnapshot>,
) -> String {
    let edits = session.edits.category(category);
    let polled_value = category_end_polled_value(session, category, fallback);
    if session.end_fields_unlocked {
        let trimmed = edits.end_input.trim();
        if !trimmed.is_empty() {
            return edits.end_input.clone();
        }
    }
    polled_value
        .map(|value| value.to_string())
        .unwrap_or_default()
}

pub(crate) fn category_start_value(
    session: &RecordingSession,
    category: RecordingCategory,
) -> Option<u64> {
    Some(category_start_source_value(session, category).unwrap_or(0))
}

pub(crate) fn category_end_value(
    session: &RecordingSession,
    category: RecordingCategory,
    fallback: Option<&RecordingSnapshot>,
) -> Option<u64> {
    let edits = session.edits.category(category);
    let polled_value = category_end_polled_value(session, category, fallback);
    if session.end_fields_unlocked {
        match parse_count_input(&edits.end_input) {
            Ok(Some(value)) => Some(value),
            Ok(None) => Some(polled_value.unwrap_or(0)),
            Err(()) => None,
        }
    } else {
        Some(polled_value.unwrap_or(0))
    }
}

pub(crate) fn sum_optional_included(
    values: impl IntoIterator<Item = (bool, Option<u64>)>,
) -> Option<u64> {
    let mut total = 0u64;
    let mut has_numeric_value = false;
    for (included, value) in values {
        if !included {
            continue;
        }
        if let Some(value) = value {
            has_numeric_value = true;
            total = total.saturating_add(value);
        }
    }
    if has_numeric_value {
        Some(total)
    } else {
        Some(0)
    }
}

pub(crate) fn counter_oids_from_walk(varbinds: &[SnmpVarBind]) -> CounterOidSet {
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
        if oid.as_slice() == PRT_MARKER_LIFECOUNT_3.as_slice() && total_seen.insert(oid.clone()) {
            total.push(oid.clone());
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

#[cfg(test)]
mod tests {
    use super::{
        ManualLineState, build_poll_label_map, category_end_display, category_end_value,
        category_start_display, category_start_value, default_recording_oid_inputs,
        default_toner_oids, delta_value, format_clock_hms_with_offset, format_elapsed_hms,
        manual_pricing_totals, manual_round_total_cents, missing_recording_snapshot_categories,
        recording_profile_from_settings_lossy, round_to_nearest_5_cents, snmp_oids,
        sum_optional_included, sum_two,
    };
    use crate::app::constants::PRT_GENERAL_PRINTER_NAME_OID;
    use crate::app::profiles::{
        MachineMatcher, ManufacturerProfile, OidLabel, RecordingOidProfile, TonerOidProfile,
    };
    use crate::app::{
        ManualFinisherLineItem, ManualFinisherType, ManualLaminateSize, ManualPaperModifier,
        ManualPricingLineItem, ManualPricingSettings, ManualPrintMode, ManualPrintSize,
        ManualRoundingMode, RecordingCategory, RecordingSession, RecordingSnapshot,
    };
    use printcountpay_core::{CounterOidSet, Oid};
    use time::UtcOffset;

    #[test]
    fn sum_two_ignores_missing_side() {
        assert_eq!(sum_two(Some(120), None), Some(120));
        assert_eq!(sum_two(None, Some(45)), Some(45));
        assert_eq!(sum_two(None, None), None);
    }

    #[test]
    fn included_sum_skips_missing_values() {
        assert_eq!(
            sum_optional_included([(true, None), (true, Some(749))]),
            Some(749)
        );
        assert_eq!(
            sum_optional_included([(true, None), (false, Some(30))]),
            Some(0)
        );
    }

    #[test]
    fn partial_totals_still_produce_delta() {
        let start_total = sum_two(None, Some(1_669_151));
        let end_total = sum_two(None, Some(1_669_900));

        assert_eq!(delta_value(start_total, end_total), Some(749));
    }

    #[test]
    fn missing_recording_snapshot_categories_skip_unconfigured_groups() {
        let snapshot = RecordingSnapshot {
            received_at: 123,
            bw_printer: Some(456),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        };
        let recording_oids = RecordingOidProfile {
            copies_bw: vec![Oid::from_slice(&[1, 3, 6, 1, 4, 1, 999, 1])],
            copies_color: Vec::new(),
            prints_bw: vec![Oid::from_slice(&[1, 3, 6, 1, 4, 1, 999, 2])],
            prints_color: Vec::new(),
        };

        assert_eq!(
            missing_recording_snapshot_categories(&snapshot, &recording_oids),
            vec![RecordingCategory::CopiesBw]
        );
    }

    #[test]
    fn missing_recording_values_display_na_but_calculate_as_zero() {
        let session = RecordingSession::default();

        assert_eq!(
            category_start_display(&session, RecordingCategory::CopiesBw),
            ""
        );
        assert_eq!(
            category_end_display(&session, RecordingCategory::CopiesBw, None),
            ""
        );
        assert_eq!(
            category_start_value(&session, RecordingCategory::CopiesBw),
            Some(0)
        );
        assert_eq!(
            category_end_value(&session, RecordingCategory::CopiesBw, None),
            Some(0)
        );
    }

    #[test]
    fn locked_end_value_ignores_manual_override() {
        let mut session = RecordingSession::default();
        session.edits.copies_bw.end_input = "1234".to_string();

        assert_eq!(
            category_end_display(&session, RecordingCategory::CopiesBw, None),
            ""
        );
        assert_eq!(
            category_end_value(&session, RecordingCategory::CopiesBw, None),
            Some(0)
        );

        session.end_fields_unlocked = true;

        assert_eq!(
            category_end_display(&session, RecordingCategory::CopiesBw, None),
            "1234"
        );
        assert_eq!(
            category_end_value(&session, RecordingCategory::CopiesBw, None),
            Some(1234)
        );
    }

    #[test]
    fn elapsed_time_formats_as_hours_minutes_seconds() {
        assert_eq!(format_elapsed_hms(0), "00:00:00");
        assert_eq!(format_elapsed_hms(59), "00:00:59");
        assert_eq!(format_elapsed_hms(3_661), "01:01:01");
    }

    #[test]
    fn clock_time_formats_as_hours_minutes_seconds() {
        assert_eq!(format_clock_hms_with_offset(0, UtcOffset::UTC), "00:00:00");
        assert_eq!(format_clock_hms_with_offset(59, UtcOffset::UTC), "00:00:59");
        assert_eq!(
            format_clock_hms_with_offset(3_661, UtcOffset::UTC),
            "01:01:01"
        );
    }

    #[test]
    fn rounding_uses_five_cent_steps() {
        assert_eq!(round_to_nearest_5_cents(0), 0);
        assert_eq!(round_to_nearest_5_cents(2), 0);
        assert_eq!(round_to_nearest_5_cents(3), 5);
        assert_eq!(round_to_nearest_5_cents(27), 25);
        assert_eq!(round_to_nearest_5_cents(28), 30);
    }

    #[test]
    fn manual_rounding_uses_floor_steps() {
        assert_eq!(
            manual_round_total_cents(1_249, ManualRoundingMode::None),
            1_249
        );
        assert_eq!(
            manual_round_total_cents(1_249, ManualRoundingMode::FiveCents),
            1_245
        );
        assert_eq!(
            manual_round_total_cents(1_249, ManualRoundingMode::HalfEuro),
            1_200
        );
        assert_eq!(
            manual_round_total_cents(1_249, ManualRoundingMode::DownToFiveEuro),
            1_000
        );
        assert_eq!(
            manual_round_total_cents(1_249, ManualRoundingMode::DownToTenEuro),
            1_000
        );
    }

    #[test]
    fn manual_pricing_counts_print_sides_and_paper_sheets_separately() {
        let settings = ManualPricingSettings {
            a3_input: "1.00".to_string(),
            modifiers: vec![ManualPaperModifier {
                name_input: "300G".to_string(),
                legacy_price_input: String::new(),
                applies_a0: true,
                a0_price_input: "2.00".to_string(),
                applies_a1: true,
                a1_price_input: "1.50".to_string(),
                applies_a2: true,
                a2_price_input: "1.25".to_string(),
                applies_a3: true,
                a3_price_input: "1.00".to_string(),
                applies_a4: true,
                a4_price_input: "0.75".to_string(),
            }],
            line_items: vec![ManualPricingLineItem {
                size: ManualPrintSize::A3,
                modifier_index: Some(0),
                sides_input: "4".to_string(),
                double_sided: true,
                ..ManualPricingLineItem::default()
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(600));
        assert_eq!(totals.discount_cents, Some(0));
        assert_eq!(totals.total_cents, Some(600));
    }

    #[test]
    fn manual_pricing_applies_cutting_discount_and_rounding_after_totals() {
        let settings = ManualPricingSettings {
            a3_input: "1.00".to_string(),
            line_items: vec![ManualPricingLineItem {
                size: ManualPrintSize::A3,
                modifier_index: None,
                sides_input: "7".to_string(),
                ..ManualPricingLineItem::default()
            }],
            cutting_enabled: true,
            discount_input: "10".to_string(),
            rounding_mode: ManualRoundingMode::DownToFiveEuro,
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(1_000));
        assert_eq!(totals.discount_cents, Some(100));
        assert_eq!(totals.total_before_rounding_cents, Some(900));
        assert_eq!(totals.total_cents, Some(500));
    }

    #[test]
    fn manual_pricing_uses_bw_tiers_for_a3() {
        let settings = ManualPricingSettings {
            a3_bw_first_input: "1.00".to_string(),
            a3_bw_next_input: "0.80".to_string(),
            a3_bw_rest_input: "0.60".to_string(),
            line_items: vec![ManualPricingLineItem {
                size: ManualPrintSize::A3,
                print_mode: ManualPrintMode::Bw,
                sides_input: "12".to_string(),
                ..ManualPricingLineItem::default()
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(1_020));
    }

    #[test]
    fn manual_pricing_uses_color_tiers_for_a4() {
        let settings = ManualPricingSettings {
            a4_color_first_input: "0.50".to_string(),
            a4_color_rest_input: "0.30".to_string(),
            line_items: vec![ManualPricingLineItem {
                size: ManualPrintSize::A4,
                print_mode: ManualPrintMode::Color,
                sides_input: "7".to_string(),
                ..ManualPricingLineItem::default()
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(310));
    }

    #[test]
    fn manual_pricing_shares_color_counter_across_sizes_and_preserves_modifier_prices() {
        let settings = ManualPricingSettings {
            a3_color_first_input: "1.50".to_string(),
            a3_color_rest_input: "1.00".to_string(),
            a4_color_first_input: "0.75".to_string(),
            a4_color_rest_input: "0.50".to_string(),
            modifiers: vec![
                ManualPaperModifier {
                    name_input: "300G".to_string(),
                    a3_price_input: "0.20".to_string(),
                    ..ManualPaperModifier::default()
                },
                ManualPaperModifier {
                    name_input: "200G".to_string(),
                    a4_price_input: "0.10".to_string(),
                    ..ManualPaperModifier::default()
                },
            ],
            line_items: vec![
                ManualPricingLineItem {
                    size: ManualPrintSize::A3,
                    print_mode: ManualPrintMode::Color,
                    modifier_index: Some(0),
                    sides_input: "10".to_string(),
                    ..ManualPricingLineItem::default()
                },
                ManualPricingLineItem {
                    size: ManualPrintSize::A4,
                    print_mode: ManualPrintMode::Color,
                    modifier_index: Some(1),
                    sides_input: "10".to_string(),
                    ..ManualPricingLineItem::default()
                },
            ],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(2_050));
        assert!(matches!(
            totals.line_states.as_slice(),
            [
                ManualLineState::Ready(first),
                ManualLineState::Ready(second)
            ] if first.print_total_cents == 1_250
                && first.paper_total_cents == 200
                && second.print_total_cents == 500
                && second.paper_total_cents == 100
        ));
    }

    #[test]
    fn manual_pricing_shares_bw_counter_across_a3_and_a4_lines() {
        let settings = ManualPricingSettings {
            a3_bw_first_input: "0.35".to_string(),
            a3_bw_next_input: "0.20".to_string(),
            a3_bw_rest_input: "0.12".to_string(),
            a4_bw_first_input: "0.25".to_string(),
            a4_bw_next_input: "0.10".to_string(),
            a4_bw_rest_input: "0.06".to_string(),
            line_items: vec![
                ManualPricingLineItem {
                    size: ManualPrintSize::A3,
                    print_mode: ManualPrintMode::Bw,
                    sides_input: "20".to_string(),
                    ..ManualPricingLineItem::default()
                },
                ManualPricingLineItem {
                    size: ManualPrintSize::A4,
                    print_mode: ManualPrintMode::Bw,
                    sides_input: "20".to_string(),
                    ..ManualPricingLineItem::default()
                },
            ],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.subtotal_cents, Some(515));
        assert!(matches!(
            totals.line_states.as_slice(),
            [
                ManualLineState::Ready(first),
                ManualLineState::Ready(second)
            ] if first.print_total_cents == 395
                && second.print_total_cents == 120
        ));
    }

    #[test]
    fn manual_pricing_adds_laminate_finishers_by_size() {
        let settings = ManualPricingSettings {
            laminate_a2_input: "2.50".to_string(),
            finisher_items: vec![ManualFinisherLineItem {
                finisher_type: ManualFinisherType::Laminate,
                laminate_size: ManualLaminateSize::A2,
                amount_input: "3".to_string(),
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.finishers_total_cents, Some(750));
        assert_eq!(totals.subtotal_cents, Some(750));
    }

    #[test]
    fn manual_pricing_adds_a0_laminate_finishers_by_size() {
        let settings = ManualPricingSettings {
            laminate_a0_input: "12.00".to_string(),
            finisher_items: vec![ManualFinisherLineItem {
                finisher_type: ManualFinisherType::Laminate,
                laminate_size: ManualLaminateSize::A0,
                amount_input: "2".to_string(),
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.finishers_total_cents, Some(2400));
        assert_eq!(totals.subtotal_cents, Some(2400));
    }

    #[test]
    fn manual_pricing_counts_binding_amounts() {
        let settings = ManualPricingSettings {
            binding_input: "4.00".to_string(),
            finisher_items: vec![ManualFinisherLineItem {
                finisher_type: ManualFinisherType::Binding,
                amount_input: "2".to_string(),
                ..ManualFinisherLineItem::default()
            }],
            ..ManualPricingSettings::default()
        };

        let totals = manual_pricing_totals(&settings);

        assert_eq!(totals.finishers_total_cents, Some(800));
        assert_eq!(totals.subtotal_cents, Some(800));
    }

    #[test]
    fn recurring_poll_omits_printer_name_oid() {
        let oids = snmp_oids(
            &CounterOidSet::default(),
            &recording_profile_from_settings_lossy(&default_recording_oid_inputs()),
            &[],
            &default_toner_oids(),
        );

        assert!(
            !oids.contains(&Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID)),
            "recurring polling should not request the discovery-only printer name OID"
        );
    }

    #[test]
    fn extra_poll_labels_do_not_override_canonical_labels() {
        let recording_settings = default_recording_oid_inputs();
        let canonical_oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 401]);
        let extra_oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 367, 3, 2, 1, 2, 19, 5, 1, 9, 71]);
        let profile = ManufacturerProfile {
            id: "test-profile".to_string(),
            manufacturer: "ricoh".to_string(),
            firmware: "test".to_string(),
            recording: recording_profile_from_settings_lossy(&recording_settings),
            counters: CounterOidSet::default(),
            toner: TonerOidProfile::default(),
            extra_poll_labels: vec![
                OidLabel {
                    oid: canonical_oid.clone(),
                    label: "Should Not Override".to_string(),
                },
                OidLabel {
                    oid: extra_oid.clone(),
                    label: "Observed counter 71".to_string(),
                },
            ],
            counter_table: None,
            legacy_profile_ids: Vec::new(),
            matchers: Vec::<MachineMatcher>::new(),
            source_path: None,
        };

        let labels = build_poll_label_map(
            &CounterOidSet::default(),
            &recording_settings,
            Some(&profile),
        );

        assert_eq!(
            labels.get(&canonical_oid).map(String::as_str),
            Some("Recording: Prints B/W")
        );
        assert_eq!(
            labels.get(&extra_oid).map(String::as_str),
            Some("Observed counter 71")
        );
    }
}

pub(crate) fn snmp_oids(
    counter_oids: &CounterOidSet,
    recording_oids: &RecordingOidProfile,
    extra_poll_oids: &[Oid],
    toner_oids: &TonerOidProfile,
) -> Vec<Oid> {
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

    for oid in &recording_oids.copies_bw {
        push(oid.clone());
    }
    for oid in &recording_oids.prints_bw {
        push(oid.clone());
    }
    for oid in &recording_oids.copies_color {
        push(oid.clone());
    }
    for oid in &recording_oids.prints_color {
        push(oid.clone());
    }

    for oid in extra_poll_oids {
        push(oid.clone());
    }

    for oid in &counter_oids.bw {
        push(oid.clone());
    }
    for oid in &counter_oids.color {
        push(oid.clone());
    }
    for oid in &counter_oids.total {
        push(oid.clone());
    }

    if let Some(oid) = toner_oids.black.clone() {
        push(oid);
    }
    if let Some(oid) = toner_oids.cyan.clone() {
        push(oid);
    }
    if let Some(oid) = toner_oids.magenta.clone() {
        push(oid);
    }
    if let Some(oid) = toner_oids.yellow.clone() {
        push(oid);
    }

    oids
}
