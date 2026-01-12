use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snmp2::{AsyncSession, Error as Snmp2Error, Oid as Snmp2Oid, Value as Snmp2Value};

use tokio::time::timeout;
use tracing::{debug, trace, warn};

use crate::targets;
use crate::{Error, SnmpAddress};

const MAX_OIDS_PER_GET: usize = 24;

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

        match async_get(address, community, oids, config).await {
            Ok(response) => {
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
            Err(error) => {
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    error = %error,
                    "SNMP GET failed"
                );
                Err(error)
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

        match async_walk(address, community, root_oid, max_results, config).await {
            Ok(response) => {
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
            Err(error) => {
                warn!(
                    target: targets::SNMP,
                    address = %address_label,
                    error = %error,
                    "SNMP WALK failed"
                );
                Err(error)
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

async fn async_get(
    address: SnmpAddress,
    community: String,
    oids: Vec<Oid>,
    config: SnmpConfig,
) -> Result<SnmpResponse, Error> {
    let address_label = address.to_string();
    let mut session = open_session(&address, &community, &config).await?;
    let snmp_oids = to_snmp2_oids(&address, &oids)?;
    let mut varbinds = Vec::new();

    for chunk in snmp_oids.chunks(MAX_OIDS_PER_GET) {
        let oid_refs: Vec<&Snmp2Oid> = chunk.iter().collect();
        varbinds.extend(
            get_many_with_retries(
                &mut session,
                &address,
                &address_label,
                &config,
                oid_refs.as_slice(),
            )
            .await?,
        );
    }

    Ok(SnmpResponse { address, varbinds })
}

async fn async_walk(
    address: SnmpAddress,
    community: String,
    root_oid: Oid,
    max_results: usize,
    config: SnmpConfig,
) -> Result<SnmpResponse, Error> {
    let address_label = address.to_string();
    let mut session = open_session(&address, &community, &config).await?;
    let root_snmp = to_snmp2_oid(&address, &root_oid)?;
    let mut current = root_snmp.clone();
    let mut results = Vec::new();
    let mut remaining = max_results;

    loop {
        if max_results > 0 {
            if remaining == 0 {
                break;
            }
            remaining -= 1;
        }

        let timeout_ms = duration_ms(config.timeout);
        let mut attempts = 0;
        let pdu = loop {
            match timeout(config.timeout, session.getnext(&current)).await {
                Ok(Ok(pdu)) => break pdu,
                Ok(Err(error)) => {
                    if attempts < config.retries {
                        attempts += 1;
                        continue;
                    }
                    return Err(map_snmp2_error(&address, error));
                }
                Err(_) => {
                    if attempts < config.retries {
                        attempts += 1;
                        continue;
                    }
                    return Err(Error::SnmpTimeout {
                        address: address.to_string(),
                        timeout_ms,
                    });
                }
            }
        };

        let mut progressed = false;
        for (oid, value) in pdu.varbinds {
            let mapped_oid = map_snmp2_oid(&address_label, &oid);
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
            if oid == current {
                return Ok(SnmpResponse {
                    address,
                    varbinds: results,
                });
            }

            results.push(SnmpVarBind {
                oid: mapped_oid,
                value: map_snmp2_value(&address_label, value),
            });
            current = oid.to_owned();
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

async fn open_session(
    address: &SnmpAddress,
    community: &str,
    config: &SnmpConfig,
) -> Result<AsyncSession, Error> {
    let timeout_ms = duration_ms(config.timeout);
    let target = format!("{}:{}", address.host, address.port);
    match timeout(
        config.timeout,
        AsyncSession::new_v2c(target.as_str(), community.as_bytes(), 0),
    )
    .await
    {
        Ok(Ok(session)) => Ok(session),
        Ok(Err(error)) => Err(map_snmp2_io_error(address, timeout_ms, error)),
        Err(_) => Err(Error::SnmpTimeout {
            address: address.to_string(),
            timeout_ms,
        }),
    }
}

async fn get_many_with_retries(
    session: &mut AsyncSession,
    address: &SnmpAddress,
    address_label: &str,
    config: &SnmpConfig,
    oids: &[&Snmp2Oid<'_>],
) -> Result<Vec<SnmpVarBind>, Error> {
    let timeout_ms = duration_ms(config.timeout);
    let mut attempts = 0;
    loop {
        match timeout(config.timeout, session.get_many(oids)).await {
            Ok(Ok(pdu)) => return Ok(map_snmp2_varbinds(address_label, pdu)),
            Ok(Err(error)) => {
                if attempts < config.retries {
                    attempts += 1;
                    continue;
                }
                return Err(map_snmp2_error(address, error));
            }
            Err(_) => {
                if attempts < config.retries {
                    attempts += 1;
                    continue;
                }
                return Err(Error::SnmpTimeout {
                    address: address.to_string(),
                    timeout_ms,
                });
            }
        }
    }
}


fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn to_snmp2_oids(address: &SnmpAddress, oids: &[Oid]) -> Result<Vec<Snmp2Oid<'static>>, Error> {
    let mut snmp_oids = Vec::new();
    for oid in oids {
        snmp_oids.push(to_snmp2_oid(address, oid)?);
    }
    Ok(snmp_oids)
}

fn to_snmp2_oid(address: &SnmpAddress, oid: &Oid) -> Result<Snmp2Oid<'static>, Error> {
    let arcs: Vec<u64> = oid.as_slice().iter().map(|value| u64::from(*value)).collect();
    Snmp2Oid::from(arcs.as_slice()).map_err(|error| Error::SnmpFailure {
        address: address.to_string(),
        details: format!("Invalid OID {oid}: {error:?}"),
    })
}

fn map_snmp2_io_error(address: &SnmpAddress, timeout_ms: u64, error: io::Error) -> Error {
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

fn map_snmp2_error(address: &SnmpAddress, error: Snmp2Error) -> Error {
    match error {
        Snmp2Error::CommunityMismatch => Error::SnmpAuth {
            address: address.to_string(),
            details: Some(format!("{error}")),
        },
        other => Error::SnmpFailure {
            address: address.to_string(),
            details: other.to_string(),
        },
    }
}

fn map_snmp2_oid(address: &str, oid: &Snmp2Oid<'_>) -> Oid {
    let Some(iter) = oid.iter() else {
        warn!(
            target: targets::SNMP,
            address = %address,
            "Failed to parse SNMP OID"
        );
        return Oid(Vec::new());
    };

    let mut arcs = Vec::new();
    for arc in iter {
        match u32::try_from(arc) {
            Ok(value) => arcs.push(value),
            Err(_) => {
                warn!(
                    target: targets::SNMP,
                    address = %address,
                    arc = arc,
                    "SNMP OID component out of range"
                );
                return Oid(Vec::new());
            }
        }
    }

    Oid(arcs)
}

fn map_snmp2_value(address: &str, value: Snmp2Value<'_>) -> SnmpValue {
    match value {
        Snmp2Value::Null => SnmpValue::Null,
        Snmp2Value::Integer(value) => SnmpValue::Integer(value),
        Snmp2Value::OctetString(value) => SnmpValue::OctetString(value.to_vec()),
        Snmp2Value::ObjectIdentifier(value) => {
            SnmpValue::ObjectIdentifier(map_snmp2_oid(address, &value))
        }
        Snmp2Value::IpAddress(value) => SnmpValue::IpAddress(value),
        Snmp2Value::Counter32(value) => SnmpValue::Counter32(value),
        Snmp2Value::Unsigned32(value) => SnmpValue::Unsigned32(value),
        Snmp2Value::Timeticks(value) => SnmpValue::Timeticks(value),
        Snmp2Value::Counter64(value) => SnmpValue::Counter64(value),
        Snmp2Value::Opaque(value) => SnmpValue::Opaque(value.to_vec()),
        Snmp2Value::EndOfMibView => SnmpValue::Other("EndOfMibView".to_string()),
        Snmp2Value::NoSuchObject => SnmpValue::Other("NoSuchObject".to_string()),
        Snmp2Value::NoSuchInstance => SnmpValue::Other("NoSuchInstance".to_string()),
        Snmp2Value::Sequence(_) => SnmpValue::Other("Sequence".to_string()),
        Snmp2Value::Set(_) => SnmpValue::Other("Set".to_string()),
        Snmp2Value::Constructed(tag, _) => {
            SnmpValue::Other(format!("Constructed({tag})"))
        }
        Snmp2Value::GetRequest(_) => SnmpValue::Other("GetRequest".to_string()),
        Snmp2Value::GetNextRequest(_) => SnmpValue::Other("GetNextRequest".to_string()),
        Snmp2Value::GetBulkRequest(_) => SnmpValue::Other("GetBulkRequest".to_string()),
        Snmp2Value::Response(_) => SnmpValue::Other("Response".to_string()),
        Snmp2Value::SetRequest(_) => SnmpValue::Other("SetRequest".to_string()),
        Snmp2Value::InformRequest(_) => SnmpValue::Other("InformRequest".to_string()),
        Snmp2Value::Trap(_) => SnmpValue::Other("Trap".to_string()),
        Snmp2Value::Report(_) => SnmpValue::Other("Report".to_string()),
        Snmp2Value::Boolean(value) => SnmpValue::Other(format!("Boolean({value})")),
    }
}

fn oid_is_descendant(root: &Oid, candidate: &Oid) -> bool {
    let root = root.as_slice();
    let candidate = candidate.as_slice();
    candidate.len() >= root.len() && candidate[..root.len()] == root[..]
}

fn map_snmp2_varbinds(address: &str, pdu: snmp2::Pdu<'_>) -> Vec<SnmpVarBind> {
    pdu.varbinds
        .map(|(oid, value)| SnmpVarBind {
            oid: map_snmp2_oid(address, &oid),
            value: map_snmp2_value(address, value),
        })
        .collect()
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
