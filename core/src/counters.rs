use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::{CounterOids, CounterSnapshot, EpochSeconds};
use crate::snmp::{Oid, SnmpVarBind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterKind {
    Bw,
    Color,
    Total,
}

impl fmt::Display for CounterKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CounterKind::Bw => f.write_str("bw"),
            CounterKind::Color => f.write_str("color"),
            CounterKind::Total => f.write_str("total"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterMode {
    BwColor,
    TotalOnly,
    Partial,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterWarning {
    Missing { kind: CounterKind },
    UsedTotalFallback,
    DerivedTotal,
    NonNumeric { kind: CounterKind, oid: String },
    Overflow { kind: CounterKind },
}

impl fmt::Display for CounterWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CounterWarning::Missing { kind } => {
                write!(f, "Missing {kind} counter")
            }
            CounterWarning::UsedTotalFallback => f.write_str("Used total counter fallback"),
            CounterWarning::DerivedTotal => f.write_str("Total counter derived from BW + Color"),
            CounterWarning::NonNumeric { kind, oid } => {
                write!(f, "Non-numeric {kind} counter at OID {oid}")
            }
            CounterWarning::Overflow { kind } => write!(f, "Overflow summing {kind} counters"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CounterOidSet {
    pub bw: Vec<Oid>,
    pub color: Vec<Oid>,
    pub total: Vec<Oid>,
    /// Sum every distinct B/W and color OID instead of using them as alternatives.
    /// All components must be present. Total OIDs always remain ordered fallbacks.
    #[serde(default)]
    pub sum_bw_color: bool,
}

#[derive(Debug, Clone)]
pub struct CounterResolution {
    pub snapshot: CounterSnapshot,
    pub mode: CounterMode,
    pub warnings: Vec<CounterWarning>,
}

pub fn resolve_counters(
    timestamp: EpochSeconds,
    oids: &CounterOidSet,
    varbinds: &[SnmpVarBind],
) -> CounterResolution {
    let mut warnings = Vec::new();

    let resolve_color = if oids.sum_bw_color {
        sum_counter_values
    } else {
        find_counter_value
    };
    let bw = resolve_color(CounterKind::Bw, &oids.bw, varbinds, &mut warnings);
    let color = resolve_color(CounterKind::Color, &oids.color, varbinds, &mut warnings);
    let total = find_counter_value(CounterKind::Total, &oids.total, varbinds, &mut warnings);

    let mut snapshot = CounterSnapshot::new(timestamp);

    snapshot.source_oids = CounterOids {
        bw: bw.oid.as_ref().map(|oid| oid.to_string()),
        color: color.oid.as_ref().map(|oid| oid.to_string()),
        total: total.oid.as_ref().map(|oid| oid.to_string()),
    };

    let mode = if bw.value.is_some() && color.value.is_some() {
        snapshot.bw = bw.value;
        snapshot.color = color.value;
        if let Some(total_value) = total.value {
            snapshot.total = Some(total_value);
        } else {
            snapshot.total = bw.value.unwrap().checked_add(color.value.unwrap());
            warnings.push(if snapshot.total.is_some() {
                CounterWarning::DerivedTotal
            } else {
                CounterWarning::Overflow {
                    kind: CounterKind::Total,
                }
            });
            snapshot.source_oids.total = None;
        }
        CounterMode::BwColor
    } else if let Some(total_value) = total.value {
        snapshot.total = Some(total_value);
        if bw.value.is_none() {
            warnings.push(CounterWarning::Missing {
                kind: CounterKind::Bw,
            });
        }
        if color.value.is_none() {
            warnings.push(CounterWarning::Missing {
                kind: CounterKind::Color,
            });
        }
        warnings.push(CounterWarning::UsedTotalFallback);
        CounterMode::TotalOnly
    } else {
        snapshot.bw = bw.value;
        snapshot.color = color.value;
        if bw.value.is_none() {
            warnings.push(CounterWarning::Missing {
                kind: CounterKind::Bw,
            });
        }
        if color.value.is_none() {
            warnings.push(CounterWarning::Missing {
                kind: CounterKind::Color,
            });
        }
        warnings.push(CounterWarning::Missing {
            kind: CounterKind::Total,
        });

        if bw.value.is_some() || color.value.is_some() {
            CounterMode::Partial
        } else {
            CounterMode::Missing
        }
    };

    CounterResolution {
        snapshot,
        mode,
        warnings,
    }
}

#[derive(Debug, Clone)]
struct CounterValue {
    value: Option<u64>,
    oid: Option<Oid>,
}

fn sum_counter_values(
    kind: CounterKind,
    components: &[Oid],
    varbinds: &[SnmpVarBind],
    warnings: &mut Vec<CounterWarning>,
) -> CounterValue {
    let missing = CounterValue {
        value: None,
        oid: None,
    };
    if components.is_empty() {
        return missing;
    }

    let mut seen = HashSet::new();
    let mut total = 0u64;
    for oid in components {
        if !seen.insert(oid) {
            continue;
        }
        let component = find_counter_value(kind, std::slice::from_ref(oid), varbinds, warnings);
        let Some(value) = component.value else {
            // A partial sum must not masquerade as the full device counter.
            return missing;
        };
        let Some(sum) = total.checked_add(value) else {
            warnings.push(CounterWarning::Overflow { kind });
            return missing;
        };
        total = sum;
    }

    CounterValue {
        value: Some(total),
        // A derived value has no single source OID.
        oid: (seen.len() == 1).then(|| components[0].clone()),
    }
}

fn find_counter_value(
    kind: CounterKind,
    candidates: &[Oid],
    varbinds: &[SnmpVarBind],
    warnings: &mut Vec<CounterWarning>,
) -> CounterValue {
    for candidate in candidates {
        if let Some(varbind) = varbinds.iter().find(|item| item.oid == *candidate) {
            if varbind.value.is_missing() {
                continue;
            }
            if let Some(value) = varbind.value.as_u64() {
                return CounterValue {
                    value: Some(value),
                    oid: Some(candidate.clone()),
                };
            }

            warnings.push(CounterWarning::NonNumeric {
                kind,
                oid: candidate.to_string(),
            });
        }
    }

    CounterValue {
        value: None,
        oid: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snmp::SnmpValue;

    fn oid(value: &str) -> Oid {
        value.parse().expect("oid")
    }

    #[test]
    fn prefers_bw_color_and_derives_total() {
        let oids = CounterOidSet {
            bw: vec![oid("1.2.3.1")],
            color: vec![oid("1.2.3.2")],
            total: vec![oid("1.2.3.3")],
            ..CounterOidSet::default()
        };
        let varbinds = vec![
            SnmpVarBind {
                oid: oid("1.2.3.1"),
                value: SnmpValue::Counter32(100),
            },
            SnmpVarBind {
                oid: oid("1.2.3.2"),
                value: SnmpValue::Counter32(50),
            },
        ];

        let resolution = resolve_counters(1_725_000_000, &oids, &varbinds);
        assert_eq!(resolution.mode, CounterMode::BwColor);
        assert_eq!(resolution.snapshot.bw, Some(100));
        assert_eq!(resolution.snapshot.color, Some(50));
        assert_eq!(resolution.snapshot.total, Some(150));
        assert!(
            resolution
                .warnings
                .iter()
                .any(|warning| matches!(warning, CounterWarning::DerivedTotal))
        );
    }

    #[test]
    fn falls_back_to_total() {
        let oids = CounterOidSet {
            bw: vec![oid("1.2.3.1")],
            color: vec![oid("1.2.3.2")],
            total: vec![oid("1.2.3.3")],
            ..CounterOidSet::default()
        };
        let varbinds = vec![SnmpVarBind {
            oid: oid("1.2.3.3"),
            value: SnmpValue::Counter32(999),
        }];

        let resolution = resolve_counters(1_725_000_000, &oids, &varbinds);
        assert_eq!(resolution.mode, CounterMode::TotalOnly);
        assert_eq!(resolution.snapshot.total, Some(999));
        assert!(
            resolution
                .warnings
                .iter()
                .any(|warning| matches!(warning, CounterWarning::UsedTotalFallback))
        );
    }

    #[test]
    fn reports_missing_counters() {
        let oids = CounterOidSet::default();
        let resolution = resolve_counters(1_725_000_000, &oids, &[]);
        assert_eq!(resolution.mode, CounterMode::Missing);
        assert!(
            resolution
                .warnings
                .iter()
                .any(|warning| matches!(warning, CounterWarning::Missing { .. }))
        );
    }

    #[test]
    fn legacy_oid_sets_keep_ordered_fallbacks() {
        let set: CounterOidSet =
            ron::from_str("(bw: [([1,2,3,1]), ([1,2,3,2])], color: [], total: [])")
                .expect("legacy mapping");
        assert!(!set.sum_bw_color);
        let varbinds = vec![
            SnmpVarBind {
                oid: oid("1.2.3.1"),
                value: SnmpValue::Counter32(12),
            },
            SnmpVarBind {
                oid: oid("1.2.3.2"),
                value: SnmpValue::Counter32(34),
            },
        ];
        assert_eq!(resolve_counters(1, &set, &varbinds).snapshot.bw, Some(12));
    }

    fn summed_set() -> CounterOidSet {
        CounterOidSet {
            bw: vec![oid("1.2.3.1"), oid("1.2.3.2"), oid("1.2.3.1")],
            color: vec![oid("1.2.3.3"), oid("1.2.3.4")],
            total: vec![oid("1.2.3.5"), oid("1.2.3.6")],
            sum_bw_color: true,
        }
    }

    fn summed_values() -> Vec<SnmpVarBind> {
        [100, 200, 0, 40, 350, 999]
            .into_iter()
            .enumerate()
            .map(|(index, value)| SnmpVarBind {
                oid: oid(&format!("1.2.3.{}", index + 1)),
                value: SnmpValue::Counter64(value),
            })
            .collect()
    }

    #[test]
    fn sums_distinct_copy_print_counters_but_keeps_total_fallbacks() {
        let set = summed_set();
        let encoded = ron::to_string(&set).expect("serialize");
        let set = ron::from_str(&encoded).expect("deserialize");
        let result = resolve_counters(1, &set, &summed_values());
        assert_eq!(result.mode, CounterMode::BwColor);
        assert_eq!(result.snapshot.bw, Some(300));
        assert_eq!(result.snapshot.color, Some(40));
        assert_eq!(result.snapshot.total, Some(350));
        assert_eq!(result.snapshot.source_oids.bw, None);
        assert_eq!(
            result.snapshot.source_oids.total.as_deref(),
            Some("1.2.3.5")
        );
    }

    #[test]
    fn incomplete_sum_never_reports_a_partial_count_as_a_total() {
        for bad_value in [
            SnmpValue::NoSuchObject,
            SnmpValue::NoSuchInstance,
            SnmpValue::Null,
            SnmpValue::OctetString(b"unknown".to_vec()),
            SnmpValue::Integer(-1),
        ] {
            let mut values = summed_values();
            values[1].value = bad_value;
            let result = resolve_counters(1, &summed_set(), &values);
            assert_eq!(result.mode, CounterMode::TotalOnly);
            assert_eq!(result.snapshot.bw, None);
            assert_eq!(result.snapshot.total, Some(350));
        }
        let mut values = summed_values();
        values.remove(1);
        assert_eq!(
            resolve_counters(1, &summed_set(), &values).snapshot.bw,
            None
        );
    }

    #[test]
    fn summed_counters_derive_total_and_preserve_zero() {
        let mut values = summed_values();
        values.truncate(4);
        let result = resolve_counters(1, &summed_set(), &values);
        assert_eq!(result.snapshot.total, Some(340));
        for value in &mut values {
            value.value = SnmpValue::Counter32(0);
        }
        let result = resolve_counters(1, &summed_set(), &values);
        assert_eq!(result.mode, CounterMode::BwColor);
        assert_eq!(result.snapshot.total, Some(0));
    }

    #[test]
    fn counter_sum_overflow_is_reported_without_wrapping() {
        let mut values = summed_values();
        values[0].value = SnmpValue::Counter64(u64::MAX);
        let result = resolve_counters(1, &summed_set(), &values);
        assert_eq!(result.snapshot.bw, None);
        assert!(result.warnings.contains(&CounterWarning::Overflow {
            kind: CounterKind::Bw
        }));

        values[1].value = SnmpValue::Counter32(0);
        values.truncate(4);
        let result = resolve_counters(1, &summed_set(), &values);
        assert_eq!(result.snapshot.total, None);
        assert!(result.warnings.contains(&CounterWarning::Overflow {
            kind: CounterKind::Total
        }));
    }
}
