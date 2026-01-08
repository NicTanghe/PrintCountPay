use crate::model::PrinterRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RicohMatch {
    NotRicoh,
    Unmapped,
    Known,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterStrategy {
    BwColorPreferred,
    BwOnly,
    TotalOnly,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterAvailability {
    pub bw: bool,
    pub color: bool,
    pub total: bool,
}

impl CounterAvailability {
    pub const NONE: CounterAvailability = CounterAvailability {
        bw: false,
        color: false,
        total: false,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RicohProfile {
    pub match_status: RicohMatch,
    pub model: Option<String>,
    pub sys_object_id: Option<String>,
    pub sys_descr: Option<String>,
    pub counters: CounterAvailability,
    pub strategy: CounterStrategy,
    pub notes: Vec<String>,
}

impl RicohProfile {
    pub fn identify(sys_object_id: Option<&str>, sys_descr: Option<&str>) -> Self {
        let sys_object_id = sys_object_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let sys_descr = sys_descr
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let ricoh_by_oid = sys_object_id.map_or(false, is_ricoh_sys_object_id);
        let ricoh_by_descr = sys_descr.map_or(false, contains_ricoh);
        let is_ricoh = ricoh_by_oid || ricoh_by_descr;

        let mut notes = Vec::new();
        if ricoh_by_oid && !ricoh_by_descr {
            notes.push("Ricoh identified via sysObjectID.".to_string());
        }
        if ricoh_by_descr && !ricoh_by_oid {
            notes.push("Ricoh identified via sysDescr.".to_string());
        }

        let model = if is_ricoh {
            sys_descr.and_then(extract_model_from_descr)
        } else {
            None
        };

        let (match_status, counters, strategy) = if !is_ricoh {
            (RicohMatch::NotRicoh, CounterAvailability::NONE, CounterStrategy::Unknown)
        } else if let Some(model) = model.as_deref() {
            match infer_color_capable(model) {
                Some(true) => (
                    RicohMatch::Known,
                    CounterAvailability {
                        bw: true,
                        color: true,
                        total: true,
                    },
                    CounterStrategy::BwColorPreferred,
                ),
                Some(false) => (
                    RicohMatch::Known,
                    CounterAvailability {
                        bw: true,
                        color: false,
                        total: true,
                    },
                    CounterStrategy::BwOnly,
                ),
                None => {
                    notes.push("Unmapped Ricoh model; counter availability unknown.".to_string());
                    (
                        RicohMatch::Unmapped,
                        CounterAvailability::NONE,
                        CounterStrategy::Unknown,
                    )
                }
            }
        } else {
            notes.push("Ricoh model string not found.".to_string());
            (
                RicohMatch::Unmapped,
                CounterAvailability::NONE,
                CounterStrategy::Unknown,
            )
        };

        RicohProfile {
            match_status,
            model,
            sys_object_id: sys_object_id.map(|value| value.to_string()),
            sys_descr: sys_descr.map(|value| value.to_string()),
            counters,
            strategy,
            notes,
        }
    }

    pub fn from_printer(record: &PrinterRecord) -> Self {
        let sys_object_id = record.sys_object_id.as_deref();
        let sys_descr = record.model.as_deref();
        Self::identify(sys_object_id, sys_descr)
    }
}

fn is_ricoh_sys_object_id(sys_object_id: &str) -> bool {
    sys_object_id.starts_with("1.3.6.1.4.1.367")
}

fn contains_ricoh(value: &str) -> bool {
    value.to_ascii_lowercase().contains("ricoh")
}

fn extract_model_from_descr(sys_descr: &str) -> Option<String> {
    let lower = sys_descr.to_ascii_lowercase();
    let key = "ricoh";
    let index = lower.find(key)?;
    let after = sys_descr[index + key.len()..].trim();
    if after.is_empty() {
        None
    } else {
        Some(after.to_string())
    }
}

fn infer_color_capable(model: &str) -> Option<bool> {
    let model = model.trim().to_ascii_lowercase();
    let color_prefixes = ["im c", "mp c", "sp c", "mpcw", "imc", "mpc", "spc"];
    for prefix in color_prefixes {
        if model.starts_with(prefix) {
            return Some(true);
        }
    }

    let mono_prefixes = ["im ", "mp ", "sp "];
    for prefix in mono_prefixes {
        if model.starts_with(prefix) {
            return Some(false);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_ricoh_from_oid() {
        let profile = RicohProfile::identify(
            Some("1.3.6.1.4.1.367.3.2.1"),
            Some("Generic Printer"),
        );
        assert_eq!(profile.match_status, RicohMatch::Unmapped);
        assert!(profile.notes.iter().any(|note| note.contains("Ricoh identified")));
    }

    #[test]
    fn identifies_color_model_from_descr() {
        let profile = RicohProfile::identify(None, Some("Ricoh IM C3000"));
        assert_eq!(profile.match_status, RicohMatch::Known);
        assert_eq!(profile.counters.color, true);
        assert_eq!(profile.strategy, CounterStrategy::BwColorPreferred);
        assert_eq!(profile.model.as_deref(), Some("IM C3000"));
    }

    #[test]
    fn identifies_mono_model_from_descr() {
        let profile = RicohProfile::identify(None, Some("RICOH IM 4000"));
        assert_eq!(profile.match_status, RicohMatch::Known);
        assert_eq!(profile.counters.color, false);
        assert_eq!(profile.strategy, CounterStrategy::BwOnly);
        assert_eq!(profile.model.as_deref(), Some("IM 4000"));
    }

    #[test]
    fn non_ricoh_is_marked() {
        let profile = RicohProfile::identify(None, Some("HP LaserJet 5000"));
        assert_eq!(profile.match_status, RicohMatch::NotRicoh);
        assert_eq!(profile.counters, CounterAvailability::NONE);
    }
}
