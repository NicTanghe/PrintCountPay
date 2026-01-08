use std::fmt;

use serde::{Deserialize, Serialize};

pub type EpochSeconds = u64;

pub const DEFAULT_SNMP_PORT: u16 = 161;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrinterId(pub String);

impl PrinterId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for PrinterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpAddress {
    pub host: String,
    #[serde(default = "default_snmp_port")]
    pub port: u16,
}

impl SnmpAddress {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn with_default_port(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: DEFAULT_SNMP_PORT,
        }
    }
}

impl fmt::Display for SnmpAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

fn default_snmp_port() -> u16 {
    DEFAULT_SNMP_PORT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrinterStatus {
    Unknown,
    Online,
    Offline,
    Error,
}

impl Default for PrinterStatus {
    fn default() -> Self {
        PrinterStatus::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrinterRecord {
    pub id: PrinterId,
    pub ip_or_hostname: Option<String>,
    pub model: Option<String>,
    pub sys_object_id: Option<String>,
    pub snmp_address: Option<SnmpAddress>,
    pub community: Option<String>,
    #[serde(default)]
    pub status: PrinterStatus,
    pub last_seen: Option<EpochSeconds>,
}

impl PrinterRecord {
    pub fn new(id: PrinterId) -> Self {
        Self {
            id,
            ip_or_hostname: None,
            model: None,
            sys_object_id: None,
            snmp_address: None,
            community: None,
            status: PrinterStatus::Unknown,
            last_seen: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterOids {
    pub bw: Option<String>,
    pub color: Option<String>,
    pub total: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CounterSnapshot {
    pub bw: Option<u64>,
    pub color: Option<u64>,
    pub total: Option<u64>,
    pub timestamp: EpochSeconds,
    #[serde(default)]
    pub source_oids: CounterOids,
}

impl CounterSnapshot {
    pub fn new(timestamp: EpochSeconds) -> Self {
        Self {
            bw: None,
            color: None,
            total: None,
            timestamp,
            source_oids: CounterOids::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printer_record_roundtrip() {
        let record = PrinterRecord {
            id: PrinterId::new("printer-001"),
            ip_or_hostname: Some("192.168.1.5".to_string()),
            model: Some("Ricoh IM C3000".to_string()),
            sys_object_id: Some("1.3.6.1.4.1.367.3.2".to_string()),
            snmp_address: Some(SnmpAddress::with_default_port("192.168.1.5")),
            community: Some("public".to_string()),
            status: PrinterStatus::Online,
            last_seen: Some(1_725_000_000),
        };

        let snapshot = CounterSnapshot {
            bw: Some(120),
            color: Some(45),
            total: Some(165),
            timestamp: 1_725_000_000,
            source_oids: CounterOids {
                bw: Some("1.3.6.1.2.1.43.10.2.1.4.1.1".to_string()),
                color: Some("1.3.6.1.2.1.43.10.2.1.4.1.2".to_string()),
                total: Some("1.3.6.1.2.1.43.10.2.1.4.1.3".to_string()),
            },
        };

        let ron = ron::ser::to_string_pretty(
            &(record, snapshot),
            ron::ser::PrettyConfig::default(),
        )
        .expect("serialize RON");
        let decoded: (PrinterRecord, CounterSnapshot) =
            ron::from_str(&ron).expect("deserialize RON");

        assert_eq!(decoded.0.status, PrinterStatus::Online);
        assert_eq!(decoded.0.snmp_address.unwrap().port, DEFAULT_SNMP_PORT);
        assert_eq!(decoded.1.total, Some(165));
    }
}
