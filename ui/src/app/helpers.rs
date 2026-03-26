use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use iced::Color;
use iced::keyboard;
use printcountpay_core::{CounterOidSet, Oid, PrinterStatus, SnmpVarBind};
use time::{OffsetDateTime, UtcOffset};

use crate::app::constants::{
    PRT_MARKER_LIFECOUNT_1, PRT_MARKER_LIFECOUNT_2, PRT_MARKER_LIFECOUNT_3,
    RICOH_BW_COPIER_COUNT_OID, RICOH_BW_PRINTER_COUNT_OID, RICOH_COLOR_COPIER_COUNT_OID,
    RICOH_COLOR_PRINTER_COUNT_OID, RICOH_COUNTER_VALUE_ROOT, RICOH_TONER_BLACK_OID,
    RICOH_TONER_CYAN_OID, RICOH_TONER_MAGENTA_OID, RICOH_TONER_YELLOW_OID, SYS_DESCR_OID,
    SYS_NAME_OID, SYS_OBJECT_ID_OID, SYS_UPTIME_OID,
};
use crate::app::profiles::{RecordingOidProfile, TonerOidProfile};
use crate::app::types::{
    BwPricing, Message, PricingSettings, RecordingCategory, RecordingOidSettings, RecordingSession,
    RecordingSnapshot, SnmpPollStatus,
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

pub(crate) fn round_to_nearest_50_cents(total_cents: u64) -> u64 {
    (total_cents + 25) / 50 * 50
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
    fallback: Option<&RecordingSnapshot>,
) -> Option<u64> {
    let edits = session.edits.category(category);
    match parse_count_input(&edits.end_input) {
        Ok(Some(value)) => Some(value),
        Ok(None) => session
            .end
            .as_ref()
            .and_then(|snapshot| snapshot_category_value(snapshot, category))
            .or_else(|| fallback.and_then(|snapshot| snapshot_category_value(snapshot, category))),
        Err(()) => None,
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
        default_recording_oid_inputs, default_toner_oids, delta_value,
        format_clock_hms_with_offset, format_elapsed_hms, recording_profile_from_settings_lossy,
        snmp_oids, sum_optional_included, sum_two,
    };
    use crate::app::constants::PRT_GENERAL_PRINTER_NAME_OID;
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
