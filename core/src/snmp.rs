use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use tokio::task;
use tracing::{debug, trace, warn};

use crate::targets;
use crate::{Error, SnmpAddress};

#[derive(Debug, Clone)]
pub struct SnmpConfig {
    pub community: String,
    pub timeout: Duration,
    pub retries: u32,
}

impl Default for SnmpConfig {
    fn default() -> Self {
        Self {
            community: "public".to_string(),
            timeout: Duration::from_secs(2),
            retries: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnmpRequest {
    pub address: SnmpAddress,
    pub community: Option<String>,
    pub oids: Vec<Oid>,
}

impl SnmpRequest {
    pub fn new(address: SnmpAddress, oids: Vec<Oid>) -> Self {
        Self {
            address,
            community: None,
            oids,
        }
    }

    pub fn with_community(mut self, community: impl Into<String>) -> Self {
        self.community = Some(community.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct SnmpWalkRequest {
    pub address: SnmpAddress,
    pub community: Option<String>,
    pub root_oid: Oid,
    pub max_results: usize,
}

impl SnmpWalkRequest {
    pub fn new(address: SnmpAddress, root_oid: Oid) -> Self {
        Self {
            address,
            community: None,
            root_oid,
            max_results: 64,
        }
    }

    pub fn with_community(mut self, community: impl Into<String>) -> Self {
        self.community = Some(community.into());
        self
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SnmpResponse {
    pub address: SnmpAddress,
    pub varbinds: Vec<SnmpVarBind>,
}

#[derive(Debug, Clone)]
pub struct SnmpVarBind {
    pub oid: Oid,
    pub value: SnmpValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Oid(pub Vec<u32>);

impl Oid {
    pub fn from_slice(slice: &[u32]) -> Self {
        Self(slice.to_vec())
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.0
    }
}

impl From<Vec<u32>> for Oid {
    fn from(value: Vec<u32>) -> Self {
        Self(value)
    }
}

impl From<&[u32]> for Oid {
    fn from(value: &[u32]) -> Self {
        Self::from_slice(value)
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for part in &self.0 {
            if !first {
                f.write_str(".")?;
            }
            first = false;
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidParseError {
    pub component: String,
}

impl fmt::Display for OidParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid OID component: {}", self.component)
    }
}

impl std::error::Error for OidParseError {}

impl FromStr for Oid {
    type Err = OidParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = Vec::new();
        for part in value.split('.') {
            if part.is_empty() {
                continue;
            }
            let parsed = part.parse::<u32>().map_err(|_| OidParseError {
                component: part.to_string(),
            })?;
            parts.push(parsed);
        }

        if parts.is_empty() {
            return Err(OidParseError {
                component: value.to_string(),
            });
        }

        Ok(Oid(parts))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SnmpValue {
    Null,
    Integer(i64),
    Unsigned32(u32),
    Counter32(u32),
    Counter64(u64),
    Timeticks(u32),
    OctetString(Vec<u8>),
    ObjectIdentifier(Oid),
    IpAddress([u8; 4]),
    Opaque(Vec<u8>),
    Other(String),
}

impl SnmpValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            SnmpValue::Unsigned32(value) => Some(u64::from(*value)),
            SnmpValue::Counter32(value) => Some(u64::from(*value)),
            SnmpValue::Counter64(value) => Some(*value),
            SnmpValue::Integer(value) => (*value >= 0).then_some(*value as u64),
            _ => None,
        }
    }

    pub fn as_text_lossy(&self) -> Option<String> {
        match self {
            SnmpValue::OctetString(bytes) | SnmpValue::Opaque(bytes) => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }
            _ => None,
        }
    }
}

impl fmt::Display for SnmpValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnmpValue::Null => f.write_str("null"),
            SnmpValue::Integer(value) => write!(f, "{value}"),
            SnmpValue::Unsigned32(value) => write!(f, "{value}"),
            SnmpValue::Counter32(value) => write!(f, "{value}"),
            SnmpValue::Counter64(value) => write!(f, "{value}"),
            SnmpValue::Timeticks(value) => write!(f, "{value} ticks"),
            SnmpValue::OctetString(bytes) | SnmpValue::Opaque(bytes) => {
                f.write_str(&String::from_utf8_lossy(bytes))
            }
            SnmpValue::ObjectIdentifier(oid) => write!(f, "{oid}"),
            SnmpValue::IpAddress(bytes) => {
                write!(f, "{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
            }
            SnmpValue::Other(value) => f.write_str(value),
        }
    }
}

pub type SnmpFuture<'a> =
    Pin<Box<dyn Future<Output = Result<SnmpResponse, Error>> + Send + 'a>>;

pub trait SnmpClient: Send + Sync {
    fn get<'a>(&'a self, request: SnmpRequest) -> SnmpFuture<'a>;
}

#[derive(Debug, Clone)]
pub struct SnmpV2cClient {
    config: SnmpConfig,
}

impl SnmpV2cClient {
    pub fn new(config: SnmpConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SnmpConfig {
        &self.config
    }

    pub async fn get(&self, request: SnmpRequest) -> Result<SnmpResponse, Error> {
        let SnmpRequest {
            address,
            community,
            oids,
        } = request;

        let config = self.config.clone();
        let community = community.unwrap_or_else(|| config.community.clone());
        let address_label = address.to_string();
        let oids_label: Vec<String> = oids.iter().map(|oid| oid.to_string()).collect();

        debug!(
            target: targets::SNMP,
            address = %address_label,
            oids = ?oids_label,
            timeout_ms = config.timeout.as_millis(),
            retries = config.retries,
            "SNMP GET"
        );

        for oid in &oids {
            trace!(target: targets::SNMP, address = %address_label, oid = %oid, "SNMP OID");
        }

        let response = task::spawn_blocking(move || {
            blocking_get(address, community, oids, config)
        })
        .await;

        match response {
            Ok(Ok(response)) => {
                debug!(
                    target: targets::SNMP,
                    address = %address_label,
                    count = response.varbinds.len(),
                    "SNMP GET ok"
                );
                for varbind in &response.varbinds {
                    trace!(
                        target: targets::SNMP,
                        address = %address_label,
                        oid = %varbind.oid,
                        value = %varbind.value,
                        "SNMP value"
                    );
                }
                Ok(response)
            }
            Ok(Err(error)) => {
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    error = %error,
                    "SNMP GET failed"
                );
                Err(error)
            }
            Err(error) => {
                let details = format!("SNMP task join error: {error}");
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    "{details}"
                );
                Err(Error::SnmpFailure {
                    address: address_label,
                    details,
                })
            }
        }
    }

    pub async fn walk(&self, request: SnmpWalkRequest) -> Result<SnmpResponse, Error> {
        let SnmpWalkRequest {
            address,
            community,
            root_oid,
            max_results,
        } = request;

        let config = self.config.clone();
        let community = community.unwrap_or_else(|| config.community.clone());
        let address_label = address.to_string();

        debug!(
            target: targets::SNMP,
            address = %address_label,
            root = %root_oid,
            max_results,
            timeout_ms = config.timeout.as_millis(),
            retries = config.retries,
            "SNMP WALK"
        );

        let response = task::spawn_blocking(move || {
            blocking_walk(address, community, root_oid, max_results, config)
        })
        .await;

        match response {
            Ok(Ok(response)) => {
                debug!(
                    target: targets::SNMP,
                    address = %address_label,
                    count = response.varbinds.len(),
                    "SNMP WALK ok"
                );
                for varbind in &response.varbinds {
                    trace!(
                        target: targets::SNMP,
                        address = %address_label,
                        oid = %varbind.oid,
                        value = %varbind.value,
                        "SNMP walk value"
                    );
                }
                Ok(response)
            }
            Ok(Err(error)) => {
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    error = %error,
                    "SNMP WALK failed"
                );
                Err(error)
            }
            Err(error) => {
                let details = format!("SNMP walk task join error: {error}");
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    "{details}"
                );
                Err(Error::SnmpFailure {
                    address: address_label,
                    details,
                })
            }
        }
    }
}

impl SnmpClient for SnmpV2cClient {
    fn get<'a>(&'a self, request: SnmpRequest) -> SnmpFuture<'a> {
        Box::pin(async move { SnmpV2cClient::get(self, request).await })
    }
}

#[derive(Debug, Clone)]
pub struct MockSnmpClient {
    config: SnmpConfig,
    queue: Arc<Mutex<VecDeque<Result<SnmpResponse, Error>>>>,
}

impl MockSnmpClient {
    pub fn new() -> Self {
        Self::with_config(SnmpConfig::default())
    }

    pub fn with_config(config: SnmpConfig) -> Self {
        Self {
            config,
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn config(&self) -> &SnmpConfig {
        &self.config
    }

    pub fn push_response(&self, response: SnmpResponse) {
        self.push_result(Ok(response));
    }

    pub fn push_error(&self, error: Error) {
        self.push_result(Err(error));
    }

    fn push_result(&self, result: Result<SnmpResponse, Error>) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.push_back(result);
        }
    }

    fn pop_result(&self) -> Option<Result<SnmpResponse, Error>> {
        if let Ok(mut queue) = self.queue.lock() {
            return queue.pop_front();
        }
        None
    }
}

impl SnmpClient for MockSnmpClient {
    fn get<'a>(&'a self, request: SnmpRequest) -> SnmpFuture<'a> {
        let address = request.address.to_string();
        Box::pin(async move {
            if let Some(result) = self.pop_result() {
                return result;
            }

            Err(Error::SnmpFailure {
                address,
                details: "MockSnmpClient queue is empty".to_string(),
            })
        })
    }
}

fn blocking_get(
    address: SnmpAddress,
    community: String,
    oids: Vec<Oid>,
    config: SnmpConfig,
) -> Result<SnmpResponse, Error> {
    let timeout_ms = duration_ms(config.timeout);
    let mut session = snmp::SyncSession::new(
        (address.host.as_str(), address.port),
        community.as_bytes(),
        Some(config.timeout),
        0,
    )
    .map_err(|error| map_snmp_io_error(&address, timeout_ms, error))?;

    let address_label = address.to_string();
    let mut varbinds = Vec::new();
    for oid in oids {
        let mut attempts = 0;
        loop {
            match session.get(oid.as_slice()) {
                Ok(response) => {
                    for (varbind_oid, varbind_val) in response.varbinds {
                        let mapped_oid = map_varbind_oid(&address_label, varbind_oid);
                        varbinds.push(SnmpVarBind {
                            oid: mapped_oid,
                            value: map_snmp_value(&address_label, varbind_val),
                        });
                    }
                    break;
                }
                Err(error) => {
                    if attempts < config.retries {
                        attempts += 1;
                        trace!(
                            target: targets::SNMP,
                            address = %address_label,
                            oid = %oid,
                            attempt = attempts,
                            "SNMP retry"
                        );
                        continue;
                    }
                    return Err(map_snmp_protocol_error(&address, timeout_ms, error));
                }
            }
        }
    }

    Ok(SnmpResponse { address, varbinds })
}

fn blocking_walk(
    address: SnmpAddress,
    community: String,
    root_oid: Oid,
    max_results: usize,
    config: SnmpConfig,
) -> Result<SnmpResponse, Error> {
    let timeout_ms = duration_ms(config.timeout);
    let mut session = snmp::SyncSession::new(
        (address.host.as_str(), address.port),
        community.as_bytes(),
        Some(config.timeout),
        0,
    )
    .map_err(|error| map_snmp_io_error(&address, timeout_ms, error))?;

    let address_label = address.to_string();
    let mut results = Vec::new();
    let mut current = root_oid.clone();

    for _ in 0..max_results {
        let response = session
            .getnext(current.as_slice())
            .map_err(|error| map_snmp_protocol_error(&address, timeout_ms, error))?;

        let mut progressed = false;
        for (varbind_oid, varbind_val) in response.varbinds {
            let mapped_oid = map_varbind_oid(&address_label, varbind_oid);
            if mapped_oid.0.is_empty() {
                return Ok(SnmpResponse {
                    address,
                    varbinds: results,
                });
            }

            if !oid_is_descendant(&root_oid, &mapped_oid) {
                return Ok(SnmpResponse {
                    address,
                    varbinds: results,
                });
            }

            if mapped_oid == current {
                return Ok(SnmpResponse {
                    address,
                    varbinds: results,
                });
            }

            results.push(SnmpVarBind {
                oid: mapped_oid.clone(),
                value: map_snmp_value(&address_label, varbind_val),
            });
            current = mapped_oid;
            progressed = true;
        }

        if !progressed {
            break;
        }
    }

    Ok(SnmpResponse {
        address,
        varbinds: results,
    })
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn map_snmp_io_error(address: &SnmpAddress, timeout_ms: u64, error: io::Error) -> Error {
    if error.kind() == io::ErrorKind::TimedOut {
        Error::SnmpTimeout {
            address: address.to_string(),
            timeout_ms,
        }
    } else {
        Error::SnmpFailure {
            address: address.to_string(),
            details: error.to_string(),
        }
    }
}

fn map_snmp_protocol_error(
    address: &SnmpAddress,
    timeout_ms: u64,
    error: snmp::SnmpError,
) -> Error {
    match error {
        snmp::SnmpError::CommunityMismatch => Error::SnmpAuth {
            address: address.to_string(),
            details: Some(format!("{error:?}")),
        },
        snmp::SnmpError::ReceiveError => Error::SnmpTimeout {
            address: address.to_string(),
            timeout_ms,
        },
        other => Error::SnmpFailure {
            address: address.to_string(),
            details: format!("{other:?}"),
        },
    }
}

fn map_snmp_value(address: &str, value: snmp::Value<'_>) -> SnmpValue {
    match value {
        snmp::Value::Null => SnmpValue::Null,
        snmp::Value::Integer(value) => SnmpValue::Integer(value),
        snmp::Value::OctetString(value) => SnmpValue::OctetString(value.to_vec()),
        snmp::Value::ObjectIdentifier(value) => match map_object_identifier(address, value) {
            Some(oid) => SnmpValue::ObjectIdentifier(oid),
            None => SnmpValue::Other("ObjectIdentifier(<unparseable>)".to_string()),
        },
        snmp::Value::IpAddress(value) => SnmpValue::IpAddress(value),
        snmp::Value::Counter32(value) => SnmpValue::Counter32(value),
        snmp::Value::Unsigned32(value) => SnmpValue::Unsigned32(value),
        snmp::Value::Timeticks(value) => SnmpValue::Timeticks(value),
        snmp::Value::Counter64(value) => SnmpValue::Counter64(value),
        snmp::Value::Opaque(value) => SnmpValue::Opaque(value.to_vec()),
        other => SnmpValue::Other(format!("{other:?}")),
    }
}

fn oid_is_descendant(root: &Oid, candidate: &Oid) -> bool {
    let root = root.as_slice();
    let candidate = candidate.as_slice();
    candidate.len() >= root.len() && candidate[..root.len()] == root[..]
}

fn map_varbind_oid(address: &str, oid: snmp::ObjectIdentifier<'_>) -> Oid {
    map_object_identifier(address, oid).unwrap_or_else(|| Oid(Vec::new()))
}

fn map_object_identifier(address: &str, oid: snmp::ObjectIdentifier<'_>) -> Option<Oid> {
    let mut buf: snmp::ObjIdBuf = [0u32; 128];
    match oid.read_name(&mut buf) {
        Ok(name) => Some(Oid(name.to_vec())),
        Err(error) => {
            warn!(
                target: targets::SNMP,
                address = %address,
                error = ?error,
                "Failed to parse SNMP OID"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oid_parses_and_formats() {
        let oid: Oid = "1.3.6.1.2.1.1.1.0".parse().expect("parse oid");
        assert_eq!(oid.to_string(), "1.3.6.1.2.1.1.1.0");
        assert_eq!(oid.as_slice(), &[1, 3, 6, 1, 2, 1, 1, 1, 0]);
    }

    fn run_future<T>(future: impl std::future::Future<Output = T>) -> T {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio runtime");
        runtime.block_on(future)
    }

    #[test]
    fn mock_snmp_returns_queued_response() {
        let mock = MockSnmpClient::new();
        let address = SnmpAddress::with_default_port("192.168.1.10");
        let oid: Oid = "1.3.6.1.2.1.1.3.0".parse().expect("oid");

        mock.push_response(SnmpResponse {
            address: address.clone(),
            varbinds: vec![SnmpVarBind {
                oid: oid.clone(),
                value: SnmpValue::Counter32(123),
            }],
        });

        let request = SnmpRequest::new(address, vec![oid]);
        let response = run_future(mock.get(request)).expect("mock response");
        assert_eq!(response.varbinds.len(), 1);
        assert_eq!(response.varbinds[0].value.as_u64(), Some(123));
    }

    #[test]
    fn mock_snmp_empty_queue_returns_error() {
        let mock = MockSnmpClient::new();
        let address = SnmpAddress::with_default_port("192.168.1.10");
        let oid: Oid = "1.3.6.1.2.1.1.3.0".parse().expect("oid");
        let request = SnmpRequest::new(address.clone(), vec![oid]);

        let error = run_future(mock.get(request)).expect_err("expected error");
        match error {
            Error::SnmpFailure {
                address: error_address,
                ..
            } => {
                assert_eq!(error_address, address.to_string());
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
