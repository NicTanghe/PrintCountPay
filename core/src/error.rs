use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SNMP authentication failed for {address}")]
    SnmpAuth {
        address: String,
        details: Option<String>,
    },
    #[error("SNMP timeout for {address}")]
    SnmpTimeout {
        address: String,
        timeout_ms: u64,
    },
    #[error("SNMP failure for {address}")]
    SnmpFailure {
        address: String,
        details: String,
    },
    #[error("Unsupported Ricoh model: {model}")]
    UnsupportedModel {
        model: String,
        sys_object_id: Option<String>,
    },
    #[error("Missing counters for {printer_id}")]
    MissingCounters {
        printer_id: String,
        missing: Vec<String>,
    },
    #[error("Counter reset detected for {printer_id}")]
    CounterReset {
        printer_id: String,
        previous: u64,
        current: u64,
    },
    #[error("Discovery failure")]
    DiscoveryFailure {
        range: Option<String>,
        details: String,
    },
    #[error("RON {action} error")]
    Ron {
        action: StorageAction,
        path: Option<String>,
        #[source]
        source: ron::Error,
    },
    #[error("Storage {action} error")]
    StorageIo {
        action: StorageAction,
        path: Option<String>,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageAction {
    Load,
    Save,
}

impl fmt::Display for StorageAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageAction::Load => f.write_str("load"),
            StorageAction::Save => f.write_str("save"),
        }
    }
}

impl Error {
    pub fn user_summary(&self) -> String {
        match self {
            Error::SnmpAuth { address, .. } => {
                format!("SNMP authentication failed for {address}.")
            }
            Error::SnmpTimeout { address, .. } => {
                format!("SNMP request timed out for {address}.")
            }
            Error::SnmpFailure { address, .. } => {
                format!("SNMP error for {address}.")
            }
            Error::UnsupportedModel { model, .. } => {
                format!("Unsupported Ricoh model: {model}.")
            }
            Error::MissingCounters { printer_id, .. } => {
                format!("Missing counters for printer {printer_id}.")
            }
            Error::CounterReset { printer_id, .. } => {
                format!("Counter reset detected for printer {printer_id}.")
            }
            Error::DiscoveryFailure { .. } => "Discovery failed.".to_string(),
            Error::Ron { action, .. } => format!("Failed to {action} configuration data."),
            Error::StorageIo { action, .. } => format!("Failed to {action} configuration file."),
        }
    }

    pub fn technical_detail(&self) -> String {
        match self {
            Error::SnmpAuth { address, details } => {
                let extra = details
                    .as_ref()
                    .map(|text| format!(" ({text})"))
                    .unwrap_or_default();
                format!("SNMP auth failed for {address}{extra}.")
            }
            Error::SnmpTimeout {
                address,
                timeout_ms,
            } => format!("SNMP timeout after {timeout_ms}ms for {address}."),
            Error::SnmpFailure { address, details } => {
                format!("SNMP failure for {address}: {details}")
            }
            Error::UnsupportedModel {
                model,
                sys_object_id,
            } => {
                let sys_id = sys_object_id
                    .as_ref()
                    .map(|id| format!(" sysObjectID={id}"))
                    .unwrap_or_default();
                format!("Unsupported model {model}.{sys_id}")
            }
            Error::MissingCounters {
                printer_id,
                missing,
            } => format!(
                "Missing counters for {printer_id}: {}.",
                missing.join(", ")
            ),
            Error::CounterReset {
                printer_id,
                previous,
                current,
            } => format!(
                "Counter reset for {printer_id}: {previous} -> {current}."
            ),
            Error::DiscoveryFailure { range, details } => {
                let range = range
                    .as_ref()
                    .map(|value| format!(" range={value}."))
                    .unwrap_or_default();
                format!("Discovery failure{range} {details}")
            }
            Error::Ron {
                action,
                path,
                source,
            } => {
                let path = path
                    .as_ref()
                    .map(|value| format!(" path={value}."))
                    .unwrap_or_default();
                format!("RON {action} error.{path} {source}")
            }
            Error::StorageIo {
                action,
                path,
                source,
            } => {
                let path = path
                    .as_ref()
                    .map(|value| format!(" path={value}."))
                    .unwrap_or_default();
                format!("Storage {action} error.{path} {source}")
            }
        }
    }
}
