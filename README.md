# IPGeolocation Rust SDK

Official Rust SDK for the IPGeolocation.io IP Location API.

Look up IPv4, IPv6, and domains with `/v3/ipgeo` and `/v3/ipgeo-bulk`. Get geolocation, company, ASN, timezone, network, hostname, abuse, user-agent, and security data from one API.

- Blocking Rust client built on `reqwest`
- Typed responses plus raw JSON and XML methods
- Cargo package `ip-geolocation-api-rust-sdk` with library import `ipgeolocation`

## Table of Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [At a Glance](#at-a-glance)
- [Get Your API Key](#get-your-api-key)
- [Authentication](#authentication)
- [Plan Behavior](#plan-behavior)
- [Client Configuration](#client-configuration)
- [Available Methods](#available-methods)
- [Request Options](#request-options)
- [Examples](#examples)
- [Response Metadata](#response-metadata)
- [Errors](#errors)
- [Troubleshooting](#troubleshooting)
- [Frequently Asked Questions](#frequently-asked-questions)
- [Links](#links)

## Install

```bash
cargo add ip-geolocation-api-rust-sdk
```

```rust
use ipgeolocation::{
    IpGeolocationClient,
    IpGeolocationClientConfig,
    LookupIpGeolocationRequest,
};
```

Cargo package: `ip-geolocation-api-rust-sdk`
Library import: `ipgeolocation`
GitHub repository: <https://github.com/IPGeolocation/ip-geolocation-api-rust-sdk>

> [!NOTE]
> The Cargo package name is `ip-geolocation-api-rust-sdk`, but the library import path is `ipgeolocation`.

## Quick Start

```rust
use ipgeolocation::{
    IpGeolocationClient,
    IpGeolocationClientConfig,
    LookupIpGeolocationRequest,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("IPGEO_API_KEY").expect("set IPGEO_API_KEY first");

    let mut client = IpGeolocationClient::new(IpGeolocationClientConfig {
        api_key: Some(api_key),
        ..IpGeolocationClientConfig::default()
    })?;

    let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
        ip: Some("8.8.8.8".to_string()),
        ..LookupIpGeolocationRequest::default()
    })?;

    if let Some(ip) = response.data.ip.as_deref() {
        println!("{ip}");
    }

    if let Some(location) = response.data.location.as_ref() {
        if let Some(country_name) = location.country_name.as_deref() {
            println!("{country_name}");
        }

        if let Some(city) = location.city.as_deref() {
            println!("{city}");
        }
    }

    if let Some(time_zone) = response.data.time_zone.as_ref() {
        if let Some(name) = time_zone.name.as_deref() {
            println!("{name}");
        }
    }

    if let Some(credits_charged) = response.metadata.credits_charged {
        println!("{credits_charged}");
    }

    client.close();
    Ok(())
}
```

## At a Glance

| Item | Value |
|------|-------|
| Cargo package | `ip-geolocation-api-rust-sdk` |
| Library import | `ipgeolocation` |
| Supported Endpoints | `/v3/ipgeo`, `/v3/ipgeo-bulk` |
| Supported Inputs | IPv4, IPv6, domain |
| Main Data Returned | Geolocation, company, ASN, timezone, network, hostname, abuse, user-agent, currency, security |
| Authentication | API key, request-origin auth for `/v3/ipgeo` only |
| Response Formats | Structured JSON, raw JSON, raw XML |
| Bulk Limit | Up to 50,000 IPs or domains per request |
| Transport | Blocking `reqwest` client |

## Get Your API Key

Create an IPGeolocation account and copy an API key from your dashboard.

1. Sign up: <https://app.ipgeolocation.io/signup>
2. Verify your email if prompted
3. Sign in: <https://app.ipgeolocation.io/login>
4. Open your dashboard: <https://app.ipgeolocation.io/dashboard>
5. Copy an API key from the `API Keys` section

For server-side Rust code, keep the API key in an environment variable or secret manager. For browser-based single lookups on paid plans, use request-origin auth instead of exposing an API key in frontend code.

## Authentication

### API Key

```rust
use ipgeolocation::{IpGeolocationClient, IpGeolocationClientConfig};

let api_key = std::env::var("IPGEO_API_KEY").expect("set IPGEO_API_KEY first");

let client = IpGeolocationClient::new(IpGeolocationClientConfig {
    api_key: Some(api_key),
    ..IpGeolocationClientConfig::default()
})?;
```

### Request-Origin Auth

```rust
use ipgeolocation::{IpGeolocationClient, IpGeolocationClientConfig};

let client = IpGeolocationClient::new(IpGeolocationClientConfig {
    request_origin: Some("https://app.example.com".to_string()),
    ..IpGeolocationClientConfig::default()
})?;
```

`request_origin` must be an absolute `http` or `https` origin with no path, query string, fragment, or userinfo.
If `request_origin` is set, the client sends it in the `Origin` header.

> [!IMPORTANT]
> Request-origin auth does not work with `/v3/ipgeo-bulk`. Bulk lookup always requires `api_key`.

> [!NOTE]
> If you set both `api_key` and `request_origin`, single lookup still uses the API key. The API key is sent as the `apiKey` query parameter, so avoid logging full request URLs.

## Plan Behavior

Feature availability depends on your plan and request parameters.

| Capability | Free | Paid |
|------------|------|------|
| Single IPv4 and IPv6 lookup | Supported | Supported |
| Domain lookup | Not supported | Supported |
| Bulk lookup | Not supported | Supported |
| Non-English `lang` | Not supported | Supported |
| Request-origin auth | Not supported | Supported for `/v3/ipgeo` only |
| Optional modules via `include` | Not supported | Supported |
| `include: ["*"]` | Base response only | All plan-available modules |

Paid plans still need `include` for optional modules. `fields` and `excludes` only trim the response. They do not turn modules on or unlock paid data.

## Client Configuration

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `api_key` | `Option<String>` | unset | Required for bulk lookup. Optional for single lookup if `request_origin` is set. |
| `request_origin` | `Option<String>` | unset | Must be an absolute `http` or `https` origin with no path, query string, fragment, or userinfo. |
| `base_url` | `String` | `https://api.ipgeolocation.io` | Override the API base URL. Must be an absolute `http` or `https` URL. |
| `connect_timeout_ms` | `u64` | `10000` | Time to open the connection. Must be greater than zero. |
| `request_timeout_ms` | `u64` | `30000` | Full-request timeout. Must be greater than zero. |

Create the client with `IpGeolocationClient::new(...)`. Config values are validated when the client is created. Request values are validated before each request is sent.

## Available Methods

| Method | Returns | Notes |
|--------|---------|-------|
| `lookup_ip_geolocation(request)` | `Result<ApiResponse<IpGeolocationResponse>, IpGeolocationError>` | Single lookup. Typed JSON response. |
| `lookup_ip_geolocation_raw(request)` | `Result<ApiResponse<String>, IpGeolocationError>` | Single lookup. Raw JSON or XML string. |
| `bulk_lookup_ip_geolocation(request)` | `Result<ApiResponse<Vec<BulkLookupResult>>, IpGeolocationError>` | Bulk lookup. Typed JSON response. |
| `bulk_lookup_ip_geolocation_raw(request)` | `Result<ApiResponse<String>, IpGeolocationError>` | Bulk lookup. Raw JSON or XML string. |
| `close()` | `()` | Marks the client closed. Closed clients cannot be reused. |

> [!NOTE]
> Typed methods support JSON only. Use the raw methods when you need XML output.

## Request Options

| Field | Applies To | Notes |
|-------|------------|-------|
| `ip` | Single lookup | IPv4, IPv6, or domain. Leave it empty for caller IP lookup. |
| `ips` | Bulk lookup | Collection of 1 to 50,000 IPs or domains. Order and duplicates are preserved. |
| `lang` | Single and bulk | One of `en`, `de`, `ru`, `ja`, `fr`, `cn`, `es`, `cs`, `it`, `ko`, `fa`, `pt`. |
| `include` | Single and bulk | Collection of module names such as `security`, `abuse`, `user_agent`, `hostname`, `liveHostname`, `hostnameFallbackLive`, `geo_accuracy`, `dma_code`, or `*`. |
| `fields` | Single and bulk | Collection of field paths to keep, for example `location.country_name` or `security.threat_score`. |
| `excludes` | Single and bulk | Collection of field paths to remove from the response. |
| `user_agent` | Single and bulk | Overrides the outbound `User-Agent` header. If you also pass a `User-Agent` header in `headers`, `user_agent` wins. |
| `headers` | Single and bulk | Extra request headers as `BTreeMap<String, String>`. |
| `output` | Single and bulk | `ResponseFormat::Json` or `ResponseFormat::Xml`. Typed methods require JSON. |

## Examples

The examples below assume you already have a configured client in scope inside a function that returns `Result<(), Box<dyn std::error::Error>>`:

```rust
use ipgeolocation::{
    BulkLookupIpGeolocationRequest,
    IpGeolocationClient,
    IpGeolocationClientConfig,
    LookupIpGeolocationRequest,
    ResponseFormat,
};

let mut client = IpGeolocationClient::new(IpGeolocationClientConfig {
    api_key: Some(std::env::var("IPGEO_API_KEY").expect("set IPGEO_API_KEY first")),
    ..IpGeolocationClientConfig::default()
})?;
```

### Caller IP

Leave `ip` empty to look up the public IP of the machine making the request.

```rust
let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest::default())?;

if let Some(ip) = response.data.ip.as_deref() {
    println!("{ip}");
}
```

### Domain Lookup

Domain lookup is a paid-plan feature.

```rust
let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
    ip: Some("ipgeolocation.io".to_string()),
    ..LookupIpGeolocationRequest::default()
})?;

if let Some(ip) = response.data.ip.as_deref() {
    println!("{ip}");
}

if let Some(domain) = response.data.domain.as_deref() {
    println!("{domain}");
}
```

### Security and Abuse

```rust
let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
    ip: Some("9.9.9.9".to_string()),
    include: vec!["security".to_string(), "abuse".to_string()],
    ..LookupIpGeolocationRequest::default()
})?;

if let Some(security) = response.data.security.as_ref() {
    println!("{:?}", security.threat_score);
    println!("{:?}", security.is_vpn);
}

if let Some(abuse) = response.data.abuse.as_ref() {
    println!("{:?}", abuse.emails);
}
```

### User-Agent Parsing

To parse a visitor user-agent string, request `user_agent` and send the visitor string in the request `User-Agent` header.

```rust
use std::collections::BTreeMap;

let mut headers = BTreeMap::new();
headers.insert(
    "User-Agent".to_string(),
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_2) AppleWebKit/601.3.9 (KHTML, like Gecko) Version/9.0.2 Safari/601.3.9".to_string(),
);

let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
    ip: Some("115.240.90.163".to_string()),
    include: vec!["user_agent".to_string()],
    headers,
    ..LookupIpGeolocationRequest::default()
})?;

if let Some(user_agent) = response.data.user_agent.as_ref() {
    println!("{:?}", user_agent.name);
    println!("{:?}", user_agent.operating_system.as_ref().and_then(|value| value.name.as_deref()));
}
```

> [!NOTE]
> The `user_agent` request field overrides the SDK's default outbound `User-Agent` header. `response.data.user_agent` is different. That field contains the parsed visitor user-agent data returned by the API.

### Filtered Response

```rust
let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
    ip: Some("8.8.8.8".to_string()),
    include: vec!["security".to_string()],
    fields: vec![
        "location.country_name".to_string(),
        "security.threat_score".to_string(),
        "security.is_vpn".to_string(),
    ],
    excludes: vec!["currency".to_string()],
    ..LookupIpGeolocationRequest::default()
})?;

if let Some(location) = response.data.location.as_ref() {
    println!("{:?}", location.country_name);
}

if let Some(security) = response.data.security.as_ref() {
    println!("{:?}", security.threat_score);
    println!("{:?}", security.is_vpn);
}

println!("{}", response.data.currency.is_none());
```

### Raw XML

```rust
let response = client.lookup_ip_geolocation_raw(&LookupIpGeolocationRequest {
    ip: Some("8.8.8.8".to_string()),
    output: ResponseFormat::Xml,
    ..LookupIpGeolocationRequest::default()
})?;

println!("{}", response.data);
```

### Bulk Lookup

```rust
let response = client.bulk_lookup_ip_geolocation(&BulkLookupIpGeolocationRequest {
    ips: vec![
        "8.8.8.8".to_string(),
        "invalid-ip".to_string(),
        "1.1.1.1".to_string(),
    ],
    include: vec!["security".to_string()],
    ..BulkLookupIpGeolocationRequest::default()
})?;

for result in &response.data {
    if result.is_success() {
        if let Some(data) = result.data.as_ref() {
            println!("{:?}", data.ip);
            println!("{:?}", data.security.as_ref().and_then(|value| value.threat_score));
        }
        continue;
    }

    if let Some(error) = result.error.as_ref() {
        println!("{:?}", error.message);
    }
}
```

### Raw Bulk JSON

```rust
let response = client.bulk_lookup_ip_geolocation_raw(&BulkLookupIpGeolocationRequest {
    ips: vec!["8.8.8.8".to_string(), "1.1.1.1".to_string()],
    ..BulkLookupIpGeolocationRequest::default()
})?;

println!("{}", response.data);
```

## Response Metadata

Every method returns `ApiResponse<T>`, where:

- `data` contains the typed object or raw response string
- `metadata` contains response details such as:
  - `credits_charged`
  - `successful_records`
  - `status_code`
  - `duration_ms`
  - `raw_headers`

Example:

```rust
println!("{}", response.metadata.status_code);
println!("{}", response.metadata.duration_ms);
println!("{:?}", response.metadata.raw_headers.get("content-type"));
```

## Errors

The SDK returns `IpGeolocationError`.

| Error Variant | When it happens |
|---------------|-----------------|
| `IpGeolocationError::Validation` | Invalid config, invalid request values, or typed XML request |
| `IpGeolocationError::Api` | API returned a non-2xx response |
| `IpGeolocationError::RequestTimeout` | Connect timeout or full-request timeout |
| `IpGeolocationError::Transport` | Network or transport failure |
| `IpGeolocationError::Serialization` | Request or response serialization failure |
| `IpGeolocationError::ClientClosed` | Request attempted after `close()` |

`ApiError` exposes:

- `status_code()`
- `message()`

Example:

```rust
use ipgeolocation::{
    IpGeolocationError,
    LookupIpGeolocationRequest,
    ResponseFormat,
};

match client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
    output: ResponseFormat::Xml,
    ..LookupIpGeolocationRequest::default()
}) {
    Ok(response) => println!("{:?}", response.data.ip),
    Err(IpGeolocationError::Validation(error)) => {
        println!("{error}");
    }
    Err(IpGeolocationError::Api(error)) => {
        println!("{}", error.status_code());
        println!("{}", error.message());
    }
    Err(error) => {
        println!("{error}");
    }
}
```

## Troubleshooting

- `cargo add ip-geolocation-api-rust-sdk`, but `use ipgeolocation::...` in code.
- Bulk lookup always requires `api_key`. `request_origin` is not enough.
- Typed methods only support JSON. Use the raw methods for XML.
- If you need security, abuse, user-agent, or hostname data, include those modules explicitly.
- `fields` and `excludes` filter the response. They do not unlock paid data.
- `request_origin` must be an origin only. Do not include a path, query string, fragment, or userinfo.
- `request_timeout_ms` is a full-request timeout, not a socket-only body-read timeout.
- This client is blocking. If your application is async-first, run it on a blocking thread or add an async client later.

## Frequently Asked Questions

<details>
<summary>Can I use this SDK without an API key?</summary>

Only for single lookup with paid-plan request-origin auth. Bulk lookup always requires an API key.
</details>

<details>
<summary>Can I request XML and still get typed models?</summary>

No. Typed methods only support JSON. Use `lookup_ip_geolocation_raw` or `bulk_lookup_ip_geolocation_raw` for XML.
</details>

<details>
<summary>Why is the Cargo package name different from the library import?</summary>

The published package name is `ip-geolocation-api-rust-sdk`, but the crate you import in Rust code is `ipgeolocation`.
</details>

<details>
<summary>Why are so many response fields Option values?</summary>

Optional fields let the SDK preserve omitted API fields instead of inventing empty values for data the API did not send.
</details>

<details>
<summary>Does domain lookup work on the free plan?</summary>

No. Domain lookup is a paid-plan feature.
</details>

<details>
<summary>Is this client async?</summary>

Not yet. The current client is blocking and uses `reqwest::blocking`.
</details>

## Links

- Homepage: <https://ipgeolocation.io>
- IP Location API product page: <https://ipgeolocation.io/ip-location-api.html>
- Documentation: <https://ipgeolocation.io/documentation/ip-location-api.html>
- Crates package: <https://crates.io/crates/ip-geolocation-api-rust-sdk>
- GitHub repository: <https://github.com/IPGeolocation/ip-geolocation-api-rust-sdk>
