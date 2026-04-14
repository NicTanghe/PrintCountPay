use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use printcountpay_core::PrinterId;
use ron::de::from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use time::{Date, Duration as TimeDuration, Month, OffsetDateTime, UtcOffset};

use super::helpers::{
    bw_cost_cents, bw_pricing_from_settings, color_cost_cents, color_price_from_settings,
};
use super::types::{PricingSettings, StatisticsRangePreset};

pub(crate) const STATISTICS_POLL_TICK: Duration = Duration::from_secs(60);
pub(crate) const STATISTICS_CLEANUP_TICK: Duration = Duration::from_millis(250);
pub(crate) const STATISTICS_BUCKET_SECS: u64 = 15 * 60;
pub(crate) const RECORDED_EUR_SERIES_KEY: &str = "series:recorded-eur";
pub(crate) const RECORDED_EUR_SERIES_LABEL: &str = "Recorded EUR";
pub(crate) const ESTIMATED_INCOME_SERIES_KEY: &str = "series:estimated-income";
pub(crate) const ESTIMATED_INCOME_SERIES_LABEL: &str = "Estimated Income";
pub(crate) const ESTIMATED_INCOME_BW_SERIES_KEY: &str = "series:estimated-income-bw";
pub(crate) const ESTIMATED_INCOME_BW_SERIES_LABEL: &str = "Estimated Income B/W";
pub(crate) const ESTIMATED_INCOME_COLOR_SERIES_KEY: &str = "series:estimated-income-color";
pub(crate) const ESTIMATED_INCOME_COLOR_SERIES_LABEL: &str = "Estimated Income Color";
pub(crate) const STATISTICS_MONTHS: [Month; 12] = [
    Month::January,
    Month::February,
    Month::March,
    Month::April,
    Month::May,
    Month::June,
    Month::July,
    Month::August,
    Month::September,
    Month::October,
    Month::November,
    Month::December,
];

const RECENT_RETENTION_SECS: u64 = 2 * 24 * 60 * 60;
const BUSINESS_START_MINUTES: u16 = 10 * 60 + 45;
const BUSINESS_END_MINUTES: u16 = 18 * 60 + 45;
const RECORDING_POINTS_PER_DAY: usize = 4;
const LEGACY_TOTAL_SERIES_LABEL: &str = "Clicks: Total";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatisticsPollMetric {
    pub(crate) oid: String,
    #[serde(default)]
    pub(crate) series_key: String,
    #[serde(default)]
    pub(crate) label: String,
    pub(crate) value: u64,
}

impl StatisticsPollMetric {
    pub(crate) fn new(oid: impl Into<String>, label: impl Into<String>, value: u64) -> Self {
        let oid = oid.into();
        let label = label.into();
        Self {
            series_key: metric_series_key(&oid, &label),
            oid,
            label,
            value,
        }
    }

    fn normalize(&mut self) {
        self.oid = self.oid.trim().to_string();
        self.label = self.label.trim().to_string();
        self.series_key = metric_series_key(&self.oid, &self.label);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatisticsPollSample {
    pub(crate) captured_at: u64,
    #[serde(default)]
    pub(crate) metrics: Vec<StatisticsPollMetric>,
    #[serde(default, rename = "total", skip_serializing_if = "Option::is_none")]
    legacy_total: Option<u64>,
}

impl StatisticsPollSample {
    fn normalize(&mut self) {
        if self.metrics.is_empty() {
            if let Some(total) = self.legacy_total.take() {
                self.metrics.push(StatisticsPollMetric::new(
                    "legacy-total",
                    LEGACY_TOTAL_SERIES_LABEL,
                    total,
                ));
            }
        } else {
            self.legacy_total = None;
        }

        for metric in &mut self.metrics {
            metric.normalize();
        }

        self.metrics.sort_by(|left, right| {
            left.series_key
                .cmp(&right.series_key)
                .then_with(|| left.oid.cmp(&right.oid))
        });
        self.metrics.dedup_by(|right, left| {
            right.series_key == left.series_key
                && right.oid == left.oid
                && right.value == left.value
        });
        self.metrics.retain(|metric| !metric.oid.is_empty());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatisticsEuroSample {
    pub(crate) captured_at: u64,
    pub(crate) total_cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PrinterStatisticsEntry {
    pub(crate) printer_id: PrinterId,
    #[serde(default)]
    pub(crate) poll_samples: Vec<StatisticsPollSample>,
    #[serde(default)]
    pub(crate) euro_samples: Vec<StatisticsEuroSample>,
}

impl Default for PrinterStatisticsEntry {
    fn default() -> Self {
        Self {
            printer_id: PrinterId::new(""),
            poll_samples: Vec::new(),
            euro_samples: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StatisticsStore {
    #[serde(default)]
    pub(crate) printers: Vec<PrinterStatisticsEntry>,
}

impl StatisticsStore {
    pub(crate) fn is_empty(&self) -> bool {
        self.printers
            .iter()
            .all(|entry| entry.poll_samples.is_empty() && entry.euro_samples.is_empty())
    }

    pub(crate) fn entry(&self, printer_id: &PrinterId) -> Option<&PrinterStatisticsEntry> {
        self.printers
            .iter()
            .find(|entry| &entry.printer_id == printer_id)
    }

    fn entry_mut(&mut self, printer_id: &PrinterId) -> &mut PrinterStatisticsEntry {
        if let Some(index) = self
            .printers
            .iter()
            .position(|entry| &entry.printer_id == printer_id)
        {
            return &mut self.printers[index];
        }

        self.printers.push(PrinterStatisticsEntry {
            printer_id: printer_id.clone(),
            ..PrinterStatisticsEntry::default()
        });
        self.printers
            .last_mut()
            .expect("statistics entry should exist after insert")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatisticsCleanupResult {
    pub(crate) revision: u64,
    pub(crate) store: StatisticsStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatisticsMergeResult {
    pub(crate) changed: bool,
    pub(crate) differs_from_incoming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatisticsSeriesDefinition {
    pub(crate) key: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DayKey {
    year: i32,
    ordinal: u16,
}

#[derive(Debug, Clone, Copy)]
struct AggregatedPoint {
    timestamp: u64,
    value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatisticsTimeWindow {
    pub(crate) start_inclusive: u64,
    pub(crate) end_exclusive: u64,
}

impl StatisticsTimeWindow {
    pub(crate) fn contains(self, epoch_seconds: u64) -> bool {
        epoch_seconds >= self.start_inclusive && epoch_seconds < self.end_exclusive
    }
}

pub(crate) fn statistics_current_local_offset() -> UtcOffset {
    UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC)
}

pub(crate) fn statistics_today_date(now: u64) -> Date {
    local_datetime(now, statistics_current_local_offset())
        .map(|datetime| datetime.date())
        .unwrap_or_else(statistics_epoch_date)
}

pub(crate) fn statistics_local_date(epoch_seconds: u64) -> Option<Date> {
    local_datetime(epoch_seconds, statistics_current_local_offset()).map(|datetime| datetime.date())
}

pub(crate) fn statistics_clamp_date(date: Date, max_date: Date) -> Date {
    date.min(max_date)
}

pub(crate) fn statistics_day_options(year: i32, month: Month, max_date: Date) -> Vec<u8> {
    let last_day = if year == max_date.year() && month == max_date.month() {
        max_date.day()
    } else {
        month.length(year)
    };

    (1..=last_day).collect()
}

pub(crate) fn statistics_date_from_components(year: i32, month: Month, day: u8) -> Date {
    let day = day.min(month.length(year));
    Date::from_calendar_date(year, month, day).unwrap_or_else(|_| statistics_epoch_date())
}

pub(crate) fn statistics_date_for_preset(
    preset: StatisticsRangePreset,
    today: Date,
) -> (Date, Date) {
    match preset {
        StatisticsRangePreset::Day => (today, today),
        StatisticsRangePreset::Week => (
            today
                .checked_sub(TimeDuration::days(6))
                .unwrap_or(Date::MIN),
            today,
        ),
        StatisticsRangePreset::Month => (shift_date_back_months(today, 1), today),
        StatisticsRangePreset::ThreeMonths => (shift_date_back_months(today, 3), today),
        StatisticsRangePreset::Year => (shift_date_back_years(today, 1), today),
        StatisticsRangePreset::Custom => (today, today),
    }
}

pub(crate) fn statistics_time_window_for_dates(
    start_date: Date,
    end_date: Date,
    now: u64,
) -> StatisticsTimeWindow {
    let offset = statistics_current_local_offset();
    let today = statistics_today_date(now);
    let end_date = statistics_clamp_date(end_date, today);
    let start_date = statistics_clamp_date(start_date.min(end_date), today);
    let start_inclusive = local_date_start_epoch(start_date, offset);
    let end_exclusive = if end_date == today {
        now.saturating_add(1)
    } else {
        end_date
            .checked_add(TimeDuration::days(1))
            .map(|next_day| local_date_start_epoch(next_day, offset))
            .unwrap_or_else(|| now.saturating_add(1))
    };

    StatisticsTimeWindow {
        start_inclusive,
        end_exclusive: end_exclusive.max(start_inclusive.saturating_add(1)),
    }
}

fn shift_date_back_months(date: Date, months: u8) -> Date {
    let total_months = date.year() * 12 + i32::from(date.month() as u8 - 1);
    let shifted = total_months - i32::from(months);
    let year = shifted.div_euclid(12);
    let month_index = shifted.rem_euclid(12) as usize;
    let month = STATISTICS_MONTHS[month_index];
    let day = date.day().min(month.length(year));

    Date::from_calendar_date(year, month, day).unwrap_or(Date::MIN)
}

fn shift_date_back_years(date: Date, years: i32) -> Date {
    let year = date.year().saturating_sub(years);
    let month = date.month();
    let day = date.day().min(month.length(year));

    Date::from_calendar_date(year, month, day).unwrap_or(Date::MIN)
}

fn local_date_start_epoch(date: Date, offset: UtcOffset) -> u64 {
    date.midnight()
        .assume_offset(offset)
        .unix_timestamp()
        .max(0) as u64
}

fn statistics_epoch_date() -> Date {
    Date::from_calendar_date(1970, Month::January, 1).expect("unix epoch date")
}

pub(crate) fn load_statistics_store(path: &Path) -> Result<StatisticsStore, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let mut store = from_str::<StatisticsStore>(&contents)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))?;
    normalize_statistics_store(&mut store);
    Ok(store)
}

pub(crate) fn write_statistics_store(path: &Path, store: &StatisticsStore) -> Result<(), String> {
    let mut store = store.clone();
    normalize_statistics_store(&mut store);

    let contents =
        to_string_pretty(&store, PrettyConfig::new()).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to prepare {}: {error}", parent.display()))?;
    }

    fs::write(path, contents)
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))
}

pub(crate) fn statistics_store_latest_timestamp(store: &StatisticsStore) -> u64 {
    store
        .printers
        .iter()
        .flat_map(|entry| {
            entry
                .poll_samples
                .iter()
                .map(|sample| sample.captured_at)
                .chain(entry.euro_samples.iter().map(|sample| sample.captured_at))
        })
        .max()
        .unwrap_or(0)
}

pub(crate) fn merge_statistics_store(
    store: &mut StatisticsStore,
    incoming: &StatisticsStore,
    prefer_incoming: bool,
) -> StatisticsMergeResult {
    let merged = merged_statistics_store(store, incoming, prefer_incoming);
    let changed = merged != *store;
    let differs_from_incoming = merged != *incoming;
    *store = merged;
    StatisticsMergeResult {
        changed,
        differs_from_incoming,
    }
}

pub(crate) fn statistics_bucket(epoch_seconds: u64) -> u64 {
    epoch_seconds / STATISTICS_BUCKET_SECS
}

pub(crate) fn statistics_poll_due(last_sample_at: Option<u64>, now: u64) -> bool {
    last_sample_at
        .map(|sample_at| statistics_bucket(sample_at) != statistics_bucket(now))
        .unwrap_or(true)
}

pub(crate) fn metric_series_key(oid: &str, label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        format!("oid:{oid}")
    } else {
        format!("label:{label}")
    }
}

pub(crate) fn append_poll_sample(
    store: &mut StatisticsStore,
    printer_id: &PrinterId,
    captured_at: u64,
    metrics: Vec<StatisticsPollMetric>,
) -> bool {
    let entry = store.entry_mut(printer_id);
    if let Some(last) = entry.poll_samples.last() {
        if statistics_bucket(last.captured_at) == statistics_bucket(captured_at) {
            return false;
        }
    }

    entry.poll_samples.push(StatisticsPollSample {
        captured_at,
        metrics,
        legacy_total: None,
    });
    true
}

pub(crate) fn append_euro_sample(
    store: &mut StatisticsStore,
    printer_id: &PrinterId,
    captured_at: u64,
    total_cents: u64,
) -> bool {
    let entry = store.entry_mut(printer_id);
    if entry
        .euro_samples
        .last()
        .is_some_and(|last| last.captured_at == captured_at && last.total_cents == total_cents)
    {
        return false;
    }

    entry.euro_samples.push(StatisticsEuroSample {
        captured_at,
        total_cents,
    });
    true
}

pub(crate) fn spawn_cleanup_worker(
    store: StatisticsStore,
    revision: u64,
    now: u64,
) -> mpsc::Receiver<StatisticsCleanupResult> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let cleaned = clean_statistics_store(store, now);
        let _ = sender.send(StatisticsCleanupResult {
            revision,
            store: cleaned,
        });
    });
    receiver
}

pub(crate) fn available_series(
    store: &StatisticsStore,
    selected_printers: &HashSet<PrinterId>,
    pricing: &PricingSettings,
    time_window: Option<StatisticsTimeWindow>,
) -> Vec<StatisticsSeriesDefinition> {
    let mut seen = BTreeSet::new();
    let mut series = Vec::new();

    for entry in store
        .printers
        .iter()
        .filter(|entry| selected_printers.contains(&entry.printer_id))
    {
        for sample in &entry.poll_samples {
            if !timestamp_matches_window(sample.captured_at, time_window) {
                continue;
            }
            for metric in &sample.metrics {
                let Some(label) = display_label_for_metric(metric) else {
                    continue;
                };

                if seen.insert(metric.series_key.clone()) {
                    series.push(StatisticsSeriesDefinition {
                        key: metric.series_key.clone(),
                        label,
                    });
                }
            }
        }

        if entry
            .euro_samples
            .iter()
            .any(|sample| timestamp_matches_window(sample.captured_at, time_window))
            && seen.insert(RECORDED_EUR_SERIES_KEY.to_string())
        {
            series.push(StatisticsSeriesDefinition {
                key: RECORDED_EUR_SERIES_KEY.to_string(),
                label: RECORDED_EUR_SERIES_LABEL.to_string(),
            });
        }
    }

    let estimated_income =
        estimated_income_availability(store, selected_printers, pricing, time_window);
    for (enabled, key, label) in [
        (
            estimated_income.bw,
            ESTIMATED_INCOME_BW_SERIES_KEY,
            ESTIMATED_INCOME_BW_SERIES_LABEL,
        ),
        (
            estimated_income.color,
            ESTIMATED_INCOME_COLOR_SERIES_KEY,
            ESTIMATED_INCOME_COLOR_SERIES_LABEL,
        ),
        (
            estimated_income.total,
            ESTIMATED_INCOME_SERIES_KEY,
            ESTIMATED_INCOME_SERIES_LABEL,
        ),
    ] {
        if enabled && seen.insert(key.to_string()) {
            series.push(StatisticsSeriesDefinition {
                key: key.to_string(),
                label: label.to_string(),
            });
        }
    }

    series.sort_by(|left, right| {
        statistics_series_sort_order(&left.label)
            .cmp(&statistics_series_sort_order(&right.label))
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.key.cmp(&right.key))
    });
    series
}

pub(crate) fn aggregate_series_points(
    store: &StatisticsStore,
    selected_printers: &HashSet<PrinterId>,
    pricing: &PricingSettings,
    series_key: &str,
    max_points: usize,
    time_window: Option<StatisticsTimeWindow>,
) -> Vec<(u64, u64)> {
    let mut buckets = BTreeMap::<u64, AggregatedPoint>::new();

    for entry in store
        .printers
        .iter()
        .filter(|entry| selected_printers.contains(&entry.printer_id))
    {
        if series_key == RECORDED_EUR_SERIES_KEY {
            for sample in &entry.euro_samples {
                if !timestamp_matches_window(sample.captured_at, time_window) {
                    continue;
                }
                let bucket = statistics_bucket(sample.captured_at);
                let point = buckets.entry(bucket).or_insert(AggregatedPoint {
                    timestamp: sample.captured_at,
                    value: 0,
                });
                point.timestamp = point.timestamp.max(sample.captured_at);
                point.value = point.value.saturating_add(sample.total_cents);
            }
            continue;
        }

        if let Some(series_value) =
            estimated_income_value_for_entry(entry, series_key, pricing, time_window)
        {
            for (captured_at, total_cents) in series_value {
                let bucket = statistics_bucket(captured_at);
                let point = buckets.entry(bucket).or_insert(AggregatedPoint {
                    timestamp: captured_at,
                    value: 0,
                });
                point.timestamp = point.timestamp.max(captured_at);
                point.value = point.value.saturating_add(total_cents);
            }
            continue;
        }

        for sample in &entry.poll_samples {
            if !timestamp_matches_window(sample.captured_at, time_window) {
                continue;
            }
            let mut sample_total = 0u64;
            let mut matched = false;
            for metric in &sample.metrics {
                if metric.series_key == series_key {
                    matched = true;
                    sample_total = sample_total.saturating_add(metric.value);
                }
            }

            if !matched {
                continue;
            }

            let bucket = statistics_bucket(sample.captured_at);
            let point = buckets.entry(bucket).or_insert(AggregatedPoint {
                timestamp: sample.captured_at,
                value: 0,
            });
            point.timestamp = point.timestamp.max(sample.captured_at);
            point.value = point.value.saturating_add(sample_total);
        }
    }

    compress_points(
        buckets
            .into_values()
            .map(|point| (point.timestamp, point.value))
            .collect::<Vec<_>>(),
        max_points,
    )
}

#[derive(Debug, Clone, Copy, Default)]
struct EstimatedIncomeAvailability {
    bw: bool,
    color: bool,
    total: bool,
}

fn estimated_income_availability(
    store: &StatisticsStore,
    selected_printers: &HashSet<PrinterId>,
    pricing: &PricingSettings,
    time_window: Option<StatisticsTimeWindow>,
) -> EstimatedIncomeAvailability {
    let bw_enabled = bw_pricing_from_settings(pricing).is_some();
    let color_enabled = color_price_from_settings(pricing).is_some();
    let mut has_bw = false;
    let mut has_color = false;

    for entry in store
        .printers
        .iter()
        .filter(|entry| selected_printers.contains(&entry.printer_id))
    {
        for sample in &entry.poll_samples {
            if !timestamp_matches_window(sample.captured_at, time_window) {
                continue;
            }
            has_bw |= sample_total_bw_count(sample).is_some();
            has_color |= sample_total_color_count(sample).is_some();
            if has_bw && has_color {
                break;
            }
        }
    }

    EstimatedIncomeAvailability {
        bw: bw_enabled && has_bw,
        color: color_enabled && has_color,
        total: (bw_enabled && has_bw) || (color_enabled && has_color),
    }
}

fn estimated_income_value_for_entry(
    entry: &PrinterStatisticsEntry,
    series_key: &str,
    pricing: &PricingSettings,
    time_window: Option<StatisticsTimeWindow>,
) -> Option<Vec<(u64, u64)>> {
    if !matches!(
        series_key,
        ESTIMATED_INCOME_BW_SERIES_KEY
            | ESTIMATED_INCOME_COLOR_SERIES_KEY
            | ESTIMATED_INCOME_SERIES_KEY
    ) {
        return None;
    }

    let mut points = Vec::new();
    let bw_pricing = bw_pricing_from_settings(pricing);
    let color_price = color_price_from_settings(pricing);

    for sample in &entry.poll_samples {
        if !timestamp_matches_window(sample.captured_at, time_window) {
            continue;
        }
        let bw_total = match sample_total_bw_count(sample) {
            Some(count) => bw_pricing.map(|pricing| bw_cost_cents(count, pricing)),
            None => None,
        };
        let color_total = match sample_total_color_count(sample) {
            Some(count) => color_price.map(|price| color_cost_cents(count, price)),
            None => None,
        };

        let total_cents = match series_key {
            ESTIMATED_INCOME_BW_SERIES_KEY => bw_total,
            ESTIMATED_INCOME_COLOR_SERIES_KEY => color_total,
            ESTIMATED_INCOME_SERIES_KEY => sum_present_values([bw_total, color_total]),
            _ => None,
        };

        if let Some(total_cents) = total_cents {
            points.push((sample.captured_at, total_cents));
        }
    }

    Some(points)
}

fn timestamp_matches_window(captured_at: u64, time_window: Option<StatisticsTimeWindow>) -> bool {
    time_window
        .map(|time_window| time_window.contains(captured_at))
        .unwrap_or(true)
}

fn sample_total_bw_count(sample: &StatisticsPollSample) -> Option<u64> {
    sum_metrics_for_label(sample, "Clicks: B/W").or_else(|| {
        sum_present_values([
            sum_metrics_for_label(sample, "Recording: Copies B/W"),
            sum_metrics_for_label(sample, "Recording: Prints B/W"),
        ])
    })
}

fn sample_total_color_count(sample: &StatisticsPollSample) -> Option<u64> {
    sum_metrics_for_label(sample, "Clicks: Color").or_else(|| {
        sum_present_values([
            sum_metrics_for_label(sample, "Recording: Copies Color"),
            sum_metrics_for_label(sample, "Recording: Prints Color"),
        ])
    })
}

fn sum_metrics_for_label(sample: &StatisticsPollSample, label: &str) -> Option<u64> {
    let canonical_target = label.trim();
    let mut total = 0u64;
    let mut matched = false;
    for metric in &sample.metrics {
        if canonical_statistics_source_label(metric) == Some(canonical_target) {
            matched = true;
            total = total.saturating_add(metric.value);
        }
    }
    matched.then_some(total)
}

fn sum_present_values<const N: usize>(values: [Option<u64>; N]) -> Option<u64> {
    let mut total = 0u64;
    let mut matched = false;
    for value in values.into_iter().flatten() {
        matched = true;
        total = total.saturating_add(value);
    }
    matched.then_some(total)
}

fn statistics_series_sort_order(label: &str) -> usize {
    match label {
        "Copies B/W" => 0,
        "Prints B/W" => 1,
        "Copies Color" => 2,
        "Prints Color" => 3,
        "Total B/W" => 4,
        "Total Color" => 5,
        ESTIMATED_INCOME_BW_SERIES_LABEL => 6,
        ESTIMATED_INCOME_COLOR_SERIES_LABEL => 7,
        ESTIMATED_INCOME_SERIES_LABEL => 8,
        RECORDED_EUR_SERIES_LABEL => 9,
        _ => usize::MAX,
    }
}

pub(crate) fn normalize_statistics_store(store: &mut StatisticsStore) {
    for entry in &mut store.printers {
        for sample in &mut entry.poll_samples {
            sample.normalize();
        }

        entry
            .poll_samples
            .sort_by(|left, right| left.captured_at.cmp(&right.captured_at));
        collapse_poll_samples_by_bucket(&mut entry.poll_samples);
        entry
            .poll_samples
            .retain(|sample| !sample.metrics.is_empty());

        entry
            .euro_samples
            .sort_by(|left, right| left.captured_at.cmp(&right.captured_at));
        entry.euro_samples.dedup_by(|right, left| {
            right.captured_at == left.captured_at && right.total_cents == left.total_cents
        });
    }

    store
        .printers
        .sort_by(|left, right| left.printer_id.0.cmp(&right.printer_id.0));
    store.printers.retain(|entry| {
        !(entry.printer_id.0.trim().is_empty()
            || (entry.poll_samples.is_empty() && entry.euro_samples.is_empty()))
    });
}

fn clean_statistics_store(mut store: StatisticsStore, now: u64) -> StatisticsStore {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    for entry in &mut store.printers {
        clean_poll_samples(&mut entry.poll_samples, now, offset);
        clean_euro_samples(&mut entry.euro_samples, now, offset);
    }
    normalize_statistics_store(&mut store);
    store
}

fn clean_poll_samples(samples: &mut Vec<StatisticsPollSample>, now: u64, offset: UtcOffset) {
    let cutoff = now.saturating_sub(RECENT_RETENTION_SECS);
    samples.retain(|sample| {
        sample.captured_at >= cutoff || within_business_hours(sample.captured_at, offset)
    });
}

fn clean_euro_samples(samples: &mut Vec<StatisticsEuroSample>, now: u64, offset: UtcOffset) {
    let cutoff = now.saturating_sub(RECENT_RETENTION_SECS);
    let mut recent = Vec::new();
    let mut older_by_day: BTreeMap<DayKey, Vec<StatisticsEuroSample>> = BTreeMap::new();

    for sample in samples.drain(..) {
        if sample.captured_at >= cutoff {
            recent.push(sample);
            continue;
        }

        if let Some(day_key) = day_key(sample.captured_at, offset) {
            older_by_day.entry(day_key).or_default().push(sample);
        }
    }

    for (_, day_samples) in &mut older_by_day {
        day_samples.sort_by(|left, right| left.captured_at.cmp(&right.captured_at));
    }

    let mut cleaned = recent;
    for (_, day_samples) in older_by_day {
        cleaned.extend(select_representative_euro_samples(
            &day_samples,
            RECORDING_POINTS_PER_DAY,
        ));
    }

    *samples = cleaned;
}

fn merged_statistics_store(
    local: &StatisticsStore,
    incoming: &StatisticsStore,
    prefer_incoming: bool,
) -> StatisticsStore {
    let mut merged_entries = BTreeMap::<String, PrinterStatisticsEntry>::new();

    for entry in &local.printers {
        merged_entries.insert(entry.printer_id.0.clone(), entry.clone());
    }

    for incoming_entry in &incoming.printers {
        merged_entries
            .entry(incoming_entry.printer_id.0.clone())
            .and_modify(|local_entry| {
                *local_entry =
                    merge_printer_statistics_entry(local_entry, incoming_entry, prefer_incoming);
            })
            .or_insert_with(|| incoming_entry.clone());
    }

    let mut merged = StatisticsStore {
        printers: merged_entries.into_values().collect(),
    };
    normalize_statistics_store(&mut merged);
    merged
}

fn merge_printer_statistics_entry(
    local: &PrinterStatisticsEntry,
    incoming: &PrinterStatisticsEntry,
    prefer_incoming: bool,
) -> PrinterStatisticsEntry {
    PrinterStatisticsEntry {
        printer_id: if local.printer_id.0.trim().is_empty() {
            incoming.printer_id.clone()
        } else {
            local.printer_id.clone()
        },
        poll_samples: merge_poll_samples(
            &local.poll_samples,
            &incoming.poll_samples,
            prefer_incoming,
        ),
        euro_samples: merge_euro_samples(
            &local.euro_samples,
            &incoming.euro_samples,
            prefer_incoming,
        ),
    }
}

fn merge_poll_samples(
    local: &[StatisticsPollSample],
    incoming: &[StatisticsPollSample],
    prefer_incoming: bool,
) -> Vec<StatisticsPollSample> {
    let mut merged = BTreeMap::<u64, StatisticsPollSample>::new();

    for sample in local {
        upsert_poll_sample(&mut merged, sample, false, prefer_incoming);
    }
    for sample in incoming {
        upsert_poll_sample(&mut merged, sample, true, prefer_incoming);
    }

    merged.into_values().collect()
}

fn upsert_poll_sample(
    merged: &mut BTreeMap<u64, StatisticsPollSample>,
    sample: &StatisticsPollSample,
    incoming_source: bool,
    prefer_incoming: bool,
) {
    let bucket = statistics_bucket(sample.captured_at);
    if let Some(current) = merged.get_mut(&bucket) {
        let prefer_candidate =
            prefer_poll_sample(current, sample, incoming_source, prefer_incoming);
        *current = merge_poll_sample(current, sample, prefer_candidate);
    } else {
        merged.insert(bucket, sample.clone());
    }
}

fn prefer_poll_sample(
    current: &StatisticsPollSample,
    candidate: &StatisticsPollSample,
    incoming_source: bool,
    prefer_incoming: bool,
) -> bool {
    if candidate.captured_at != current.captured_at {
        return candidate.captured_at > current.captured_at;
    }
    if candidate.metrics.len() != current.metrics.len() {
        return candidate.metrics.len() > current.metrics.len();
    }

    incoming_source && prefer_incoming
}

fn merge_poll_sample(
    current: &StatisticsPollSample,
    candidate: &StatisticsPollSample,
    prefer_candidate: bool,
) -> StatisticsPollSample {
    let (preferred, secondary) = if prefer_candidate {
        (candidate, current)
    } else {
        (current, candidate)
    };

    let mut merged = preferred.clone();
    merged.captured_at = preferred.captured_at.max(secondary.captured_at);

    let mut metrics = BTreeMap::<(String, String), StatisticsPollMetric>::new();
    for metric in &secondary.metrics {
        metrics.insert(
            (metric.series_key.clone(), metric.oid.clone()),
            metric.clone(),
        );
    }
    for metric in &preferred.metrics {
        metrics.insert(
            (metric.series_key.clone(), metric.oid.clone()),
            metric.clone(),
        );
    }
    merged.metrics = metrics.into_values().collect();
    merged.normalize();
    merged
}

fn collapse_poll_samples_by_bucket(samples: &mut Vec<StatisticsPollSample>) {
    let mut collapsed = Vec::<StatisticsPollSample>::with_capacity(samples.len());
    for sample in samples.drain(..) {
        if let Some(current) = collapsed.last_mut()
            && statistics_bucket(current.captured_at) == statistics_bucket(sample.captured_at)
        {
            let prefer_candidate = sample.captured_at > current.captured_at
                || (sample.captured_at == current.captured_at
                    && sample.metrics.len() >= current.metrics.len());
            *current = merge_poll_sample(current, &sample, prefer_candidate);
        } else {
            collapsed.push(sample);
        }
    }
    *samples = collapsed;
}

fn merge_euro_samples(
    local: &[StatisticsEuroSample],
    incoming: &[StatisticsEuroSample],
    prefer_incoming: bool,
) -> Vec<StatisticsEuroSample> {
    let mut merged = BTreeMap::<u64, StatisticsEuroSample>::new();

    for sample in local {
        merged.insert(sample.captured_at, sample.clone());
    }
    for sample in incoming {
        merged
            .entry(sample.captured_at)
            .and_modify(|current| {
                if prefer_incoming && current.total_cents != sample.total_cents {
                    *current = sample.clone();
                }
            })
            .or_insert_with(|| sample.clone());
    }

    merged.into_values().collect()
}

fn compress_points(points: Vec<(u64, u64)>, max_points: usize) -> Vec<(u64, u64)> {
    if max_points == 0 || points.is_empty() {
        return Vec::new();
    }
    if points.len() <= max_points {
        return points;
    }

    let bucket_size = (points.len() + max_points - 1) / max_points;
    let mut buckets = Vec::new();
    for chunk in points.chunks(bucket_size) {
        if let Some((timestamp, value)) = chunk.last().copied() {
            buckets.push((timestamp, value));
        }
    }
    buckets
}

fn select_representative_euro_samples(
    samples: &[StatisticsEuroSample],
    max_points: usize,
) -> Vec<StatisticsEuroSample> {
    if samples.len() <= max_points {
        return samples.to_vec();
    }
    if max_points == 0 {
        return Vec::new();
    }
    if max_points == 1 {
        return samples
            .first()
            .cloned()
            .into_iter()
            .collect::<Vec<StatisticsEuroSample>>();
    }

    let last_index = samples.len() - 1;
    let mut selected = Vec::new();
    let mut last_pushed = None::<usize>;

    for slot in 0..max_points {
        let index = ((slot * last_index) + (max_points - 1) / 2) / (max_points - 1);
        if last_pushed == Some(index) {
            continue;
        }
        selected.push(samples[index].clone());
        last_pushed = Some(index);
    }

    selected
}

fn display_label_for_metric(metric: &StatisticsPollMetric) -> Option<String> {
    canonical_statistics_source_label(metric)
        .and_then(canonical_statistics_label)
        .map(str::to_string)
}

fn canonical_statistics_source_label(metric: &StatisticsPollMetric) -> Option<&'static str> {
    match metric.label.trim() {
        "Recording: Copies B/W" => Some("Recording: Copies B/W"),
        "Recording: Prints B/W" => Some("Recording: Prints B/W"),
        "Recording: Copies Color" => Some("Recording: Copies Color"),
        "Recording: Prints Color" => Some("Recording: Prints Color"),
        "Clicks: B/W" => Some("Clicks: B/W"),
        "Clicks: Color" => Some("Clicks: Color"),
        // Legacy labels from older profiles.
        "Copy B/W counter" => Some("Recording: Copies B/W"),
        "Copy color counter" => Some("Recording: Copies Color"),
        "Print B/W counter" => Some("Recording: Prints B/W"),
        "Print color counter" => Some("Recording: Prints Color"),
        _ => None,
    }
}

fn canonical_statistics_label(label: &str) -> Option<&'static str> {
    match label {
        "Recording: Copies B/W" => Some("Copies B/W"),
        "Recording: Prints B/W" => Some("Prints B/W"),
        "Recording: Copies Color" => Some("Copies Color"),
        "Recording: Prints Color" => Some("Prints Color"),
        "Clicks: B/W" => Some("Total B/W"),
        "Clicks: Color" => Some("Total Color"),
        _ => None,
    }
}

fn within_business_hours(epoch_seconds: u64, offset: UtcOffset) -> bool {
    let Some(minutes) = minutes_since_midnight(epoch_seconds, offset) else {
        return false;
    };
    (BUSINESS_START_MINUTES..=BUSINESS_END_MINUTES).contains(&minutes)
}

fn minutes_since_midnight(epoch_seconds: u64, offset: UtcOffset) -> Option<u16> {
    let datetime = local_datetime(epoch_seconds, offset)?;
    let hour = u16::from(datetime.hour());
    let minute = u16::from(datetime.minute());
    Some(hour * 60 + minute)
}

fn day_key(epoch_seconds: u64, offset: UtcOffset) -> Option<DayKey> {
    let datetime = local_datetime(epoch_seconds, offset)?;
    Some(DayKey {
        year: datetime.year(),
        ordinal: datetime.ordinal(),
    })
}

fn local_datetime(epoch_seconds: u64, offset: UtcOffset) -> Option<OffsetDateTime> {
    if epoch_seconds > i64::MAX as u64 {
        return None;
    }

    OffsetDateTime::from_unix_timestamp(epoch_seconds as i64)
        .ok()
        .map(|datetime| datetime.to_offset(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer_id(value: &str) -> PrinterId {
        PrinterId::new(value)
    }

    #[test]
    fn poll_sample_only_saves_once_per_bucket() {
        let mut store = StatisticsStore::default();
        let printer_id = printer_id("printer-a");
        let metrics = vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 100)];

        assert!(append_poll_sample(
            &mut store,
            &printer_id,
            900,
            metrics.clone()
        ));
        assert!(!append_poll_sample(
            &mut store,
            &printer_id,
            905,
            metrics.clone()
        ));
        assert!(append_poll_sample(&mut store, &printer_id, 1_800, metrics));

        let entry = store.entry(&printer_id).expect("statistics entry");
        assert_eq!(entry.poll_samples.len(), 2);
        assert_eq!(entry.poll_samples[0].metrics[0].value, 100);
    }

    #[test]
    fn available_series_collects_poll_metrics_and_recorded_eur() {
        let printer_id = printer_id("printer-a");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![StatisticsPollSample {
                    captured_at: 900,
                    metrics: vec![
                        StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 100),
                        StatisticsPollMetric::new("1.2.4", "Recording: Prints B/W", 50),
                        StatisticsPollMetric::new("1.2.5", "Toner: Black", 80),
                    ],
                    legacy_total: None,
                }],
                euro_samples: vec![StatisticsEuroSample {
                    captured_at: 1_000,
                    total_cents: 500,
                }],
            }],
        };
        let selected = HashSet::from([printer_id]);

        let series = available_series(&store, &selected, &pricing, None);
        assert_eq!(series.len(), 5);
        assert!(series.iter().any(|entry| entry.label == "Total B/W"));
        assert!(series.iter().any(|entry| entry.label == "Prints B/W"));
        assert!(
            series
                .iter()
                .any(|entry| entry.label == ESTIMATED_INCOME_BW_SERIES_LABEL)
        );
        assert!(
            series
                .iter()
                .any(|entry| entry.label == ESTIMATED_INCOME_SERIES_LABEL)
        );
        assert!(
            series
                .iter()
                .any(|entry| entry.label == RECORDED_EUR_SERIES_LABEL)
        );
    }

    #[test]
    fn available_series_accepts_legacy_poll_labels() {
        let printer_id = printer_id("printer-a");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![StatisticsPollSample {
                    captured_at: 900,
                    metrics: vec![
                        StatisticsPollMetric::new("1.2.3", "Print B/W counter", 100),
                        StatisticsPollMetric::new("1.2.4", "Copy color counter", 50),
                    ],
                    legacy_total: None,
                }],
                euro_samples: Vec::new(),
            }],
        };
        let selected = HashSet::from([printer_id]);

        let series = available_series(&store, &selected, &pricing, None);
        assert!(series.iter().any(|entry| entry.label == "Prints B/W"));
        assert!(series.iter().any(|entry| entry.label == "Copies Color"));
    }

    #[test]
    fn available_series_ignores_observed_counter_5_when_prints_bw_exists() {
        let printer_id = printer_id("printer-a");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![StatisticsPollSample {
                    captured_at: 900,
                    metrics: vec![
                        StatisticsPollMetric::new(
                            "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.5",
                            "Observed counter 5",
                            42,
                        ),
                        StatisticsPollMetric::new(
                            "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.11",
                            "Recording: Prints B/W",
                            84,
                        ),
                    ],
                    legacy_total: None,
                }],
                euro_samples: Vec::new(),
            }],
        };
        let selected = HashSet::from([printer_id]);

        let series = available_series(&store, &selected, &pricing, None);
        let prints_bw_count = series
            .iter()
            .filter(|entry| entry.label == "Prints B/W")
            .count();

        assert_eq!(prints_bw_count, 1);
        assert!(
            !series
                .iter()
                .any(|entry| entry.key == "label:Observed counter 5")
        );
    }

    #[test]
    fn aggregate_series_points_adds_matching_metrics_across_printers() {
        let printer_a = printer_id("printer-a");
        let printer_b = printer_id("printer-b");
        let metric_key = metric_series_key("1.2.3", "Clicks: Total");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![
                PrinterStatisticsEntry {
                    printer_id: printer_a.clone(),
                    poll_samples: vec![StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 100)],
                        legacy_total: None,
                    }],
                    euro_samples: Vec::new(),
                },
                PrinterStatisticsEntry {
                    printer_id: printer_b.clone(),
                    poll_samples: vec![StatisticsPollSample {
                        captured_at: 901,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 40)],
                        legacy_total: None,
                    }],
                    euro_samples: Vec::new(),
                },
            ],
        };
        let selected = HashSet::from([printer_a, printer_b]);

        let points = aggregate_series_points(&store, &selected, &pricing, &metric_key, 32, None);
        assert_eq!(points, vec![(901, 140)]);
    }

    #[test]
    fn aggregate_series_points_supports_estimated_income() {
        let printer_a = printer_id("printer-a");
        let printer_b = printer_id("printer-b");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![
                PrinterStatisticsEntry {
                    printer_id: printer_a.clone(),
                    poll_samples: vec![StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 10)],
                        legacy_total: None,
                    }],
                    euro_samples: Vec::new(),
                },
                PrinterStatisticsEntry {
                    printer_id: printer_b.clone(),
                    poll_samples: vec![StatisticsPollSample {
                        captured_at: 901,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 5)],
                        legacy_total: None,
                    }],
                    euro_samples: Vec::new(),
                },
            ],
        };
        let selected = HashSet::from([printer_a, printer_b]);

        let points = aggregate_series_points(
            &store,
            &selected,
            &pricing,
            ESTIMATED_INCOME_BW_SERIES_KEY,
            32,
            None,
        );
        assert_eq!(points, vec![(901, 300)]);
    }

    #[test]
    fn sample_total_bw_count_does_not_map_observed_counter_5_as_prints_bw() {
        let sample = StatisticsPollSample {
            captured_at: 900,
            metrics: vec![StatisticsPollMetric::new(
                "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.5",
                "Observed counter 5",
                42,
            )],
            legacy_total: None,
        };

        assert_eq!(sample_total_bw_count(&sample), None);
    }

    #[test]
    fn available_series_ignores_samples_outside_time_window() {
        let printer_id = printer_id("printer-a");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![
                    StatisticsPollSample {
                        captured_at: 100,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 10)],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new(
                            "1.2.4",
                            "Recording: Prints B/W",
                            5,
                        )],
                        legacy_total: None,
                    },
                ],
                euro_samples: vec![StatisticsEuroSample {
                    captured_at: 120,
                    total_cents: 500,
                }],
            }],
        };
        let selected = HashSet::from([printer_id]);
        let window = StatisticsTimeWindow {
            start_inclusive: 800,
            end_exclusive: 1_000,
        };

        let series = available_series(&store, &selected, &pricing, Some(window));

        assert_eq!(series.len(), 3);
        assert!(series.iter().any(|entry| entry.label == "Prints B/W"));
        assert!(
            series
                .iter()
                .any(|entry| entry.label == ESTIMATED_INCOME_BW_SERIES_LABEL)
        );
        assert!(
            series
                .iter()
                .any(|entry| entry.label == ESTIMATED_INCOME_SERIES_LABEL)
        );
        assert!(!series.iter().any(|entry| entry.label == "Total B/W"));
        assert!(
            !series
                .iter()
                .any(|entry| entry.label == RECORDED_EUR_SERIES_LABEL)
        );
    }

    #[test]
    fn aggregate_series_points_respects_time_window() {
        let printer_id = printer_id("printer-a");
        let metric_key = metric_series_key("1.2.3", "Clicks: Total");
        let pricing = PricingSettings::default();
        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![
                    StatisticsPollSample {
                        captured_at: 100,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 25)],
                        legacy_total: None,
                    },
                ],
                euro_samples: Vec::new(),
            }],
        };
        let selected = HashSet::from([printer_id]);
        let window = StatisticsTimeWindow {
            start_inclusive: 800,
            end_exclusive: 1_000,
        };

        let points =
            aggregate_series_points(&store, &selected, &pricing, &metric_key, 32, Some(window));

        assert_eq!(points, vec![(900, 25)]);
    }

    #[test]
    fn cleanup_keeps_recent_polls_and_discards_old_night_polls() {
        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let base = OffsetDateTime::from_unix_timestamp(1_713_744_300)
            .expect("base timestamp")
            .to_offset(offset);
        let day_start = base
            .replace_hour(0)
            .expect("hour")
            .replace_minute(0)
            .expect("minute")
            .replace_second(0)
            .expect("second")
            .replace_nanosecond(0)
            .expect("nanos");
        let old_inside_window = day_start
            .replace_hour(10)
            .expect("hour")
            .replace_minute(45)
            .expect("minute")
            .unix_timestamp() as u64;
        let old_outside_window = day_start.unix_timestamp() as u64;
        let now = old_inside_window + RECENT_RETENTION_SECS + 120;
        let recent = now - 60;

        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id("printer-a"),
                poll_samples: vec![
                    StatisticsPollSample {
                        captured_at: old_inside_window,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: old_outside_window,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 5)],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: recent,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 25)],
                        legacy_total: None,
                    },
                ],
                euro_samples: Vec::new(),
            }],
        };

        let cleaned = clean_statistics_store(store, now);
        let entry = cleaned
            .entry(&printer_id("printer-a"))
            .expect("statistics entry");

        assert_eq!(entry.poll_samples.len(), 2);
        assert!(
            entry
                .poll_samples
                .iter()
                .flat_map(|sample| sample.metrics.iter())
                .any(|metric| metric.value == 10)
        );
        assert!(
            entry
                .poll_samples
                .iter()
                .flat_map(|sample| sample.metrics.iter())
                .any(|metric| metric.value == 25)
        );
    }

    #[test]
    fn cleanup_reduces_old_recording_samples_to_four_points_per_day() {
        let offset = UtcOffset::UTC;
        let base = OffsetDateTime::from_unix_timestamp(1_713_744_300)
            .expect("base timestamp")
            .to_offset(offset);
        let day_start = base
            .replace_hour(0)
            .expect("hour")
            .replace_minute(0)
            .expect("minute")
            .replace_second(0)
            .expect("second")
            .replace_nanosecond(0)
            .expect("nanos")
            .unix_timestamp() as u64;
        let now = day_start + 24 * 60 * 60 + RECENT_RETENTION_SECS + 60;

        let samples = (0..8)
            .map(|index| StatisticsEuroSample {
                captured_at: day_start + index * 60 * 60,
                total_cents: 100 + index,
            })
            .collect::<Vec<_>>();

        let store = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id("printer-a"),
                poll_samples: Vec::new(),
                euro_samples: samples,
            }],
        };

        let cleaned = clean_statistics_store(store, now);
        let entry = cleaned
            .entry(&printer_id("printer-a"))
            .expect("statistics entry");

        assert_eq!(entry.euro_samples.len(), 4);
        assert_eq!(
            entry.euro_samples.first().map(|sample| sample.total_cents),
            Some(100)
        );
        assert_eq!(
            entry.euro_samples.last().map(|sample| sample.total_cents),
            Some(107)
        );
    }

    #[test]
    fn merge_statistics_store_prefers_newer_bucket_sample_and_keeps_history() {
        let printer_id = printer_id("printer-a");
        let mut local = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![
                    StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: 3_600,
                        metrics: vec![StatisticsPollMetric::new("1.2.4", "Clicks: Total", 20)],
                        legacy_total: None,
                    },
                ],
                euro_samples: vec![StatisticsEuroSample {
                    captured_at: 10_000,
                    total_cents: 150,
                }],
            }],
        };
        let incoming = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![
                    StatisticsPollSample {
                        captured_at: 901,
                        metrics: vec![
                            StatisticsPollMetric::new("1.2.3", "Clicks: Total", 11),
                            StatisticsPollMetric::new("1.2.5", "Toner: Black", 70),
                        ],
                        legacy_total: None,
                    },
                    StatisticsPollSample {
                        captured_at: 7_200,
                        metrics: vec![StatisticsPollMetric::new("1.2.6", "Clicks: Total", 30)],
                        legacy_total: None,
                    },
                ],
                euro_samples: vec![
                    StatisticsEuroSample {
                        captured_at: 10_000,
                        total_cents: 175,
                    },
                    StatisticsEuroSample {
                        captured_at: 11_000,
                        total_cents: 200,
                    },
                ],
            }],
        };

        let result = merge_statistics_store(&mut local, &incoming, true);
        let entry = local.entry(&printer_id).expect("statistics entry");

        assert!(result.changed);
        assert_eq!(entry.poll_samples.len(), 3);
        assert_eq!(entry.poll_samples[0].captured_at, 901);
        assert_eq!(entry.poll_samples[0].metrics.len(), 2);
        assert_eq!(entry.poll_samples[1].captured_at, 3_600);
        assert_eq!(entry.poll_samples[2].captured_at, 7_200);
        assert_eq!(entry.euro_samples.len(), 2);
        assert_eq!(entry.euro_samples[0].total_cents, 175);
        assert_eq!(entry.euro_samples[1].total_cents, 200);
    }

    #[test]
    fn merge_statistics_store_keeps_local_sample_when_it_is_newer() {
        let printer_id = printer_id("printer-a");
        let mut local = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![StatisticsPollSample {
                    captured_at: 905,
                    metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 12)],
                    legacy_total: None,
                }],
                euro_samples: vec![StatisticsEuroSample {
                    captured_at: 10_000,
                    total_cents: 190,
                }],
            }],
        };
        let incoming = StatisticsStore {
            printers: vec![PrinterStatisticsEntry {
                printer_id: printer_id.clone(),
                poll_samples: vec![StatisticsPollSample {
                    captured_at: 900,
                    metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
                    legacy_total: None,
                }],
                euro_samples: vec![StatisticsEuroSample {
                    captured_at: 10_000,
                    total_cents: 150,
                }],
            }],
        };

        let result = merge_statistics_store(&mut local, &incoming, false);
        let entry = local.entry(&printer_id).expect("statistics entry");

        assert!(result.differs_from_incoming);
        assert_eq!(entry.poll_samples.len(), 1);
        assert_eq!(entry.poll_samples[0].captured_at, 905);
        assert_eq!(entry.poll_samples[0].metrics[0].value, 12);
        assert_eq!(entry.euro_samples.len(), 1);
        assert_eq!(entry.euro_samples[0].total_cents, 190);
    }

    #[test]
    fn statistics_store_latest_timestamp_tracks_newest_poll_or_euro_sample() {
        let store = StatisticsStore {
            printers: vec![
                PrinterStatisticsEntry {
                    printer_id: printer_id("printer-a"),
                    poll_samples: vec![StatisticsPollSample {
                        captured_at: 900,
                        metrics: vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
                        legacy_total: None,
                    }],
                    euro_samples: Vec::new(),
                },
                PrinterStatisticsEntry {
                    printer_id: printer_id("printer-b"),
                    poll_samples: Vec::new(),
                    euro_samples: vec![StatisticsEuroSample {
                        captured_at: 1_200,
                        total_cents: 250,
                    }],
                },
            ],
        };

        assert_eq!(statistics_store_latest_timestamp(&store), 1_200);
    }
}
