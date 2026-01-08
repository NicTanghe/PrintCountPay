SNMP client

Types live in core:

- SnmpConfig: community, timeout, retries
- SnmpRequest: address, community override, OIDs
- SnmpResponse: address, varbinds
- SnmpValue: typed SNMP value
- SnmpClient trait: async get(request) -> response
- SnmpV2cClient: real SNMP v2c implementation

Example (real client)

```rust
use printcountpay_core::{Oid, SnmpAddress, SnmpConfig, SnmpRequest, SnmpV2cClient};

let client = SnmpV2cClient::new(SnmpConfig::default());
let oid: Oid = "1.3.6.1.2.1.1.1.0".parse().expect("oid");
let request = SnmpRequest::new(SnmpAddress::with_default_port("192.168.1.10"), vec![oid]);
let response = client.get(request).await.expect("snmp ok");
```

Mocking

Use MockSnmpClient to return deterministic results without a printer.

```rust
use printcountpay_core::{
    MockSnmpClient, Oid, SnmpAddress, SnmpRequest, SnmpResponse, SnmpVarBind, SnmpValue,
};

let mock = MockSnmpClient::new();
let oid: Oid = "1.3.6.1.2.1.1.3.0".parse().expect("oid");
mock.push_response(SnmpResponse {
    address: SnmpAddress::with_default_port("192.168.1.10"),
    varbinds: vec![SnmpVarBind {
        oid,
        value: SnmpValue::Counter32(123),
    }],
});

let request = SnmpRequest::new(
    SnmpAddress::with_default_port("192.168.1.10"),
    vec!["1.3.6.1.2.1.1.3.0".parse().expect("oid")],
);
let response = mock.get(request).await.expect("mock ok");
```
