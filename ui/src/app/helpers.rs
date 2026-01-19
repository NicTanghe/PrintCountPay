use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use iced::keyboard;
use iced::Color;
use printcountpay_core::{
    CounterOidSet, Oid, PrinterRecord, PrinterStatus, SnmpVarBind,
};

use crate::app::constants::{
    PRT_GENERAL_PRINTER_NAME_OID, PRT_MARKER_LIFECOUNT_1, PRT_MARKER_LIFECOUNT_2,
    PRT_MARKER_LIFECOUNT_3, RICOH_BW_COPIER_COUNT_OID, RICOH_BW_PRINTER_COUNT_OID,
    RICOH_COLOR_COPIER_COUNT_OID, RICOH_COLOR_PRINTER_COUNT_OID, RICOH_TONER_BLACK_OID,
    RICOH_TONER_CYAN_OID, RICOH_TONER_MAGENTA_OID, RICOH_TONER_YELLOW_OID, SYS_DESCR_OID,
    SYS_NAME_OID, SYS_OBJECT_ID_OID, SYS_UPTIME_OID,
};
use crate::app::types::{
    BwPricing, Message, PricingSettings, RecordingCategory, RecordingSession, RecordingSnapshot,
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
        keyboard::Key::Named(keyboard::key::Named::Delete) => {
            Some(Message::DeleteSelectedPrinter)
        }
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

pub(crate) fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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

pub(crate) fn format_oid_list(oids: &[Oid]) -> String {
    oids.iter()
        .map(|oid| oid.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

pub(crate) fn format_counter_oids(oids: &CounterOidSet) -> (String, String, String) {
    (
        format_oid_list(&oids.bw),
        format_oid_list(&oids.color),
        format_oid_list(&oids.total),
    )
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

pub(crate) fn extract_text(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<String> {
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

pub(crate) fn extract_value_string(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<String> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    if let Some(value) = varbind.value.as_u64() {
        return Some(value.to_string());
    }
    Some(varbind.value.to_string())
}

pub(crate) fn extract_counter_u64(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<u64> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    varbind.value.as_u64()
}

pub(crate) fn delta_value(start: Option<u64>, end: Option<u64>) -> Option<u64> {
    let start = start?;
    let end = end?;
    end.checked_sub(start)
}

pub(crate) fn sum_two(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    Some(left? + right?)
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

pub(crate) fn round_to_nearest_50_cents(total_cents: u64) -> u64 {
    (total_cents + 25) / 50 * 50
}

pub(crate) fn format_cents(cents: u64) -> String {
    let euros = cents / 100;
    let remainder = cents % 100;
    format!("{euros}.{remainder:02} EUR")
}

pub(crate) fn format_count(value: Option<u64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| "N/A".to_string())
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

pub(crate) fn category_start_value(
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

pub(crate) fn category_end_value(
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

pub(crate) fn sum_optional_included(
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

pub(crate) fn snmp_oids(counter_oids: &CounterOidSet) -> Vec<Oid> {
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

pub(crate) fn seed_printers() -> Vec<PrinterRecord> {
    Vec::new()
}
