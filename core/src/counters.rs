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
}

impl fmt::Display for CounterWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CounterWarning::Missing { kind } => {
                write!(f, "Missing {kind} counter")
            }
            CounterWarning::UsedTotalFallback => {
                f.write_str("Used total counter fallback")
            }
            CounterWarning::DerivedTotal => {
                f.write_str("Total counter derived from BW + Color")
            }
            CounterWarning::NonNumeric { kind, oid } => {
                write!(f, "Non-numeric {kind} counter at OID {oid}")
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CounterOidSet {
    pub bw: Vec<Oid>,
    pub color: Vec<Oid>,
    pub total: Vec<Oid>,
}

#[derive(Debug, Clone)]
pub struct CounterResolution {
    pub snapshot: CounterSnapshot,
    pub mode: CounterMode,
    pub warnings: Vec<CounterWarning>,
    pub raw_varbinds: Vec<SnmpVarBind>,
}

pub fn resolve_counters(
    timestamp: EpochSeconds,
    oids: &CounterOidSet,
    varbinds: &[SnmpVarBind],
) -> CounterResolution {
    let raw_varbinds = varbinds.to_vec();
    let mut warnings = Vec::new();

    let bw = find_counter_value(CounterKind::Bw, &oids.bw, varbinds, &mut warnings);
    let color = find_counter_value(CounterKind::Color, &oids.color, varbinds, &mut warnings);
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
            snapshot.total = Some(bw.value.unwrap() + color.value.unwrap());
            warnings.push(CounterWarning::DerivedTotal);
            snapshot.source_oids.total = None;
        }
        CounterMode::BwColor
    } else if let Some(total_value) = total.value {
        snapshot.total = Some(total_value);
        if bw.value.is_none() {
            warnings.push(CounterWarning::Missing { kind: CounterKind::Bw });
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
            warnings.push(CounterWarning::Missing { kind: CounterKind::Bw });
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
        raw_varbinds,
    }
}

#[derive(Debug, Clone)]
struct CounterValue {
    value: Option<u64>,
    oid: Option<Oid>,
}

fn find_counter_value(
    kind: CounterKind,
    candidates: &[Oid],
    varbinds: &[SnmpVarBind],
    warnings: &mut Vec<CounterWarning>,
) -> CounterValue {
    for candidate in candidates {
        if let Some(varbind) = varbinds.iter().find(|item| item.oid == *candidate) {
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

    CounterValue { value: None, oid: None }
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
        assert!(resolution
            .warnings
            .iter()
            .any(|warning| matches!(warning, CounterWarning::DerivedTotal)));
    }

    #[test]
    fn falls_back_to_total() {
        let oids = CounterOidSet {
            bw: vec![oid("1.2.3.1")],
            color: vec![oid("1.2.3.2")],
            total: vec![oid("1.2.3.3")],
        };
        let varbinds = vec![SnmpVarBind {
            oid: oid("1.2.3.3"),
            value: SnmpValue::Counter32(999),
        }];

        let resolution = resolve_counters(1_725_000_000, &oids, &varbinds);
        assert_eq!(resolution.mode, CounterMode::TotalOnly);
        assert_eq!(resolution.snapshot.total, Some(999));
        assert!(resolution
            .warnings
            .iter()
            .any(|warning| matches!(warning, CounterWarning::UsedTotalFallback)));
    }

    #[test]
    fn reports_missing_counters() {
        let oids = CounterOidSet::default();
        let resolution = resolve_counters(1_725_000_000, &oids, &[]);
        assert_eq!(resolution.mode, CounterMode::Missing);
        assert!(resolution
            .warnings
            .iter()
            .any(|warning| matches!(warning, CounterWarning::Missing { .. })));
    }
}
