use std::fmt;
use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

use get_if_addrs::{get_if_addrs, IfAddr};
use tracing::{debug, info, warn};

use crate::model::{EpochSeconds, PrinterId, PrinterRecord, PrinterStatus, SnmpAddress};
use crate::snmp::{Oid, SnmpConfig, SnmpRequest, SnmpV2cClient, SnmpValue, SnmpVarBind};
use crate::{targets, Error};

const SYS_DESCR_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 1, 0];
const SYS_OBJECT_ID_OID: [u32; 9] = [1, 3, 6, 1, 2, 1, 1, 2, 0];
const PRT_GENERAL_PRINTER_NAME_OID: [u32; 12] = [1, 3, 6, 1, 2, 1, 43, 5, 1, 1, 16, 1];
const PRT_MARKER_LIFECOUNT_1_OID: [u32; 13] = [1, 3, 6, 1, 2, 1, 43, 10, 2, 1, 4, 1, 1];

const FALLBACK_KEYWORDS: [&str; 14] = [
    "printer",
    "mfp",
    "ricoh",
    "xerox",
    "canon",
    "hp",
    "hewlett",
    "lexmark",
    "konica",
    "kyocera",
    "brother",
    "epson",
    "sharp",
    "samsung",
];

#[derive(Debug, Clone)]
pub struct CidrRange {
    start: u32,
    end: u32,
    network: Ipv4Addr,
    prefix: u8,
}

#[derive(Debug, Clone)]
pub struct CidrParseError {
    details: String,
}

impl fmt::Display for CidrParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.details)
    }
}

impl CidrRange {
    pub fn parse(value: &str) -> Result<Self, CidrParseError> {
        let value = value.trim();
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| CidrParseError {
                details: "CIDR must include a /prefix".to_string(),
            })?;
        let ip: Ipv4Addr = addr.parse().map_err(|_| CidrParseError {
            details: format!("Invalid IPv4 address: {addr}"),
        })?;
        let prefix: u8 = prefix.parse().map_err(|_| CidrParseError {
            details: format!("Invalid prefix length: {prefix}"),
        })?;
        if prefix > 32 {
            return Err(CidrParseError {
                details: format!("Prefix length out of range: {prefix}"),
            });
        }

        let mask = prefix_to_mask(prefix);
        let ip_u32 = ipv4_to_u32(ip);
        let network_u32 = ip_u32 & mask;
        let broadcast_u32 = network_u32 | !mask;

        let (start, end) = if prefix <= 30 {
            (network_u32 + 1, broadcast_u32.saturating_sub(1))
        } else {
            (network_u32, broadcast_u32)
        };

        Ok(Self {
            start,
            end,
            network: u32_to_ipv4(network_u32),
            prefix,
        })
    }

    pub fn iter(&self) -> CidrIter {
        CidrIter {
            current: self.start,
            end: self.end,
        }
    }

    pub fn host_count(&self) -> u32 {
        if self.end < self.start {
            0
        } else {
            self.end - self.start + 1
        }
    }

    pub fn network(&self) -> Ipv4Addr {
        self.network
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }
}

pub struct CidrIter {
    current: u32,
    end: u32,
}

impl Iterator for CidrIter {
    type Item = Ipv4Addr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }
        let ip = u32_to_ipv4(self.current);
        self.current = self.current.saturating_add(1);
        Some(ip)
    }
}

pub fn default_discovery_cidr() -> Option<String> {
    let interfaces = get_if_addrs().ok()?;
    for iface in interfaces {
        let addr = match iface.addr {
            IfAddr::V4(v4) => v4,
            _ => continue,
        };
        if addr.ip.is_loopback() {
            continue;
        }
        let octets = addr.ip.octets();
        if octets[0] == 169 && octets[1] == 254 {
            continue;
        }

        let Some(prefix) = netmask_to_prefix(addr.netmask) else {
            continue;
        };
        if prefix == 32 {
            continue;
        }
        let mask = prefix_to_mask(prefix);
        let network = ipv4_to_u32(addr.ip) & mask;
        return Some(format!("{}/{}", u32_to_ipv4(network), prefix));
    }
    None
}

pub async fn probe_printer(
    address: SnmpAddress,
    community: Option<String>,
    config: SnmpConfig,
) -> Result<Option<PrinterRecord>, Error> {
    let mut request = SnmpRequest::new(
        address.clone(),
        vec![
            Oid::from_slice(&SYS_DESCR_OID),
            Oid::from_slice(&SYS_OBJECT_ID_OID),
        ],
    );

    let community = community.filter(|value| !value.trim().is_empty());
    if let Some(value) = community.as_ref() {
        request = request.with_community(value.clone());
    }

    debug!(
        target: targets::DISCOVERY,
        address = %address,
        "Discovery probe"
    );

    let client = SnmpV2cClient::new(config);
    let response = client.get(request).await?;
    let sys_descr = extract_text(&response.varbinds, &Oid::from_slice(&SYS_DESCR_OID));
    let sys_object_id = extract_object_id(&response.varbinds, &Oid::from_slice(&SYS_OBJECT_ID_OID));

    let printer_name = probe_printer_name(&client, &address, community.as_deref()).await;
    let marker_present = if printer_name.is_none() {
        probe_marker_life_count(&client, &address, community.as_deref()).await
    } else {
        false
    };

    let fallback_match = sys_descr
        .as_deref()
        .map(is_printer_keyword_match)
        .unwrap_or(false);

    let is_printer = printer_name.is_some() || marker_present || fallback_match;
    if !is_printer {
        return Ok(None);
    }

    let model = printer_name.or(sys_descr.clone());
    let sys_object_id_text = sys_object_id.as_ref().map(ToString::to_string);
    let last_seen = Some(now_epoch_seconds());

    info!(
        target: targets::DISCOVERY,
        address = %address,
        printer = true,
        fallback = fallback_match,
        "Printer discovered"
    );

    Ok(Some(PrinterRecord {
        id: PrinterId::new(format!("snmp-{}", address.host)),
        ip_or_hostname: Some(address.host.clone()),
        model,
        sys_object_id: sys_object_id_text,
        snmp_address: Some(address),
        community,
        status: PrinterStatus::Online,
        last_seen,
    }))
}

async fn probe_printer_name(
    client: &SnmpV2cClient,
    address: &SnmpAddress,
    community: Option<&str>,
) -> Option<String> {
    let mut request = SnmpRequest::new(
        address.clone(),
        vec![Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID)],
    );
    if let Some(value) = community {
        request = request.with_community(value.to_string());
    }

    match client.get(request).await {
        Ok(response) => {
            extract_text(&response.varbinds, &Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID))
        }
        Err(error) => {
            debug!(
                target: targets::DISCOVERY,
                address = %address,
                error = %error,
                "Printer name probe failed"
            );
            None
        }
    }
}

async fn probe_marker_life_count(
    client: &SnmpV2cClient,
    address: &SnmpAddress,
    community: Option<&str>,
) -> bool {
    let mut request = SnmpRequest::new(
        address.clone(),
        vec![Oid::from_slice(&PRT_MARKER_LIFECOUNT_1_OID)],
    );
    if let Some(value) = community {
        request = request.with_community(value.to_string());
    }

    match client.get(request).await {
        Ok(response) => {
            extract_numeric(&response.varbinds, &Oid::from_slice(&PRT_MARKER_LIFECOUNT_1_OID))
                .is_some()
        }
        Err(error) => {
            debug!(
                target: targets::DISCOVERY,
                address = %address,
                error = %error,
                "Marker life count probe failed"
            );
            false
        }
    }
}

fn extract_text(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<String> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    if varbind.value.is_missing() {
        return None;
    }
    varbind
        .value
        .as_text_lossy()
        .or_else(|| Some(varbind.value.to_string()))
}

fn extract_numeric(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<u64> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    varbind.value.as_u64()
}

fn extract_object_id(varbinds: &[SnmpVarBind], oid: &Oid) -> Option<Oid> {
    let varbind = varbinds.iter().find(|varbind| varbind.oid == *oid)?;
    match &varbind.value {
        SnmpValue::ObjectIdentifier(value) => Some(value.clone()),
        _ => None,
    }
}

fn is_printer_keyword_match(value: &str) -> bool {
    let value = value.to_lowercase();
    FALLBACK_KEYWORDS
        .iter()
        .any(|keyword| value.contains(keyword))
}

fn now_epoch_seconds() -> EpochSeconds {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn netmask_to_prefix(mask: Ipv4Addr) -> Option<u8> {
    let mask_u32 = ipv4_to_u32(mask);
    let ones = mask_u32.count_ones();
    let prefix = u8::try_from(ones).ok()?;
    if prefix > 32 {
        return None;
    }
    let expected = prefix_to_mask(prefix);
    if mask_u32 == expected {
        Some(prefix)
    } else {
        warn!(
            target: targets::DISCOVERY,
            mask = %mask,
            "Non-contiguous netmask ignored"
        );
        None
    }
}

fn prefix_to_mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}

fn ipv4_to_u32(ip: Ipv4Addr) -> u32 {
    u32::from_be_bytes(ip.octets())
}

fn u32_to_ipv4(value: u32) -> Ipv4Addr {
    Ipv4Addr::from(value)
}
