use ipgeolocation::{
    BulkLookupIpGeolocationRequest, IpGeolocationClient, IpGeolocationClientConfig,
};
use serde_json::Value;

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn live_client() -> Result<Option<IpGeolocationClient>, Box<dyn std::error::Error>> {
    if !env_flag("IPGEO_RUN_LIVE_HARDENING") {
        return Ok(None);
    }

    let Some(api_key) = env_value("IPGEO_PAID_KEY") else {
        return Ok(None);
    };

    let client = IpGeolocationClient::new(IpGeolocationClientConfig {
        api_key: Some(api_key),
        ..IpGeolocationClientConfig::default()
    })?;

    Ok(Some(client))
}

#[test]
fn live_bulk_success_parity_keeps_current_fields() -> Result<(), Box<dyn std::error::Error>> {
    let Some(client) = live_client()? else {
        return Ok(());
    };

    let request = BulkLookupIpGeolocationRequest {
        ips: vec!["8.8.8.8".to_string()],
        include: vec!["security".to_string()],
        ..BulkLookupIpGeolocationRequest::default()
    };

    let typed = client.bulk_lookup_ip_geolocation(&request)?;
    let raw = client.bulk_lookup_ip_geolocation_raw(&request)?;
    let raw_items = serde_json::from_str::<Vec<Value>>(&raw.data)?;

    assert_eq!(typed.data.len(), 1);
    assert_eq!(raw_items.len(), 1);

    let typed_success = typed.data[0].data.as_ref().expect("expected success item");
    let raw_success = raw_items[0].as_object().expect("expected success object");

    assert_eq!(
        raw_success.get("ip").and_then(Value::as_str),
        typed_success.ip.as_deref()
    );
    assert_eq!(
        raw_success.get("hostname").and_then(Value::as_str),
        typed_success.hostname.as_deref()
    );
    assert_eq!(
        raw_success
            .get("asn")
            .and_then(|value| value.get("as_number"))
            .and_then(Value::as_str),
        typed_success
            .asn
            .as_ref()
            .and_then(|value| value.as_number.as_deref())
    );
    assert_eq!(
        raw_success
            .get("company")
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str),
        typed_success
            .company
            .as_ref()
            .and_then(|value| value.name.as_deref())
    );
    assert_eq!(
        raw_success
            .get("time_zone")
            .and_then(|value| value.get("current_tz_abbreviation"))
            .and_then(Value::as_str),
        typed_success
            .time_zone
            .as_ref()
            .and_then(|value| value.current_tz_abbreviation.as_deref())
    );
    assert_eq!(
        raw_success
            .get("security")
            .and_then(|value| value.get("is_proxy"))
            .and_then(Value::as_bool),
        typed_success
            .security
            .as_ref()
            .and_then(|value| value.is_proxy)
    );
    assert_eq!(
        raw_success
            .get("location")
            .and_then(|value| value.get("country_name"))
            .and_then(Value::as_str),
        typed_success
            .location
            .as_ref()
            .and_then(|value| value.country_name.as_deref())
    );

    Ok(())
}

#[test]
fn live_bulk_error_parity_keeps_message() -> Result<(), Box<dyn std::error::Error>> {
    let Some(client) = live_client()? else {
        return Ok(());
    };

    let request = BulkLookupIpGeolocationRequest {
        ips: vec!["invalid-ip".to_string()],
        ..BulkLookupIpGeolocationRequest::default()
    };

    let typed = client.bulk_lookup_ip_geolocation(&request)?;
    let raw = client.bulk_lookup_ip_geolocation_raw(&request)?;
    let raw_items = serde_json::from_str::<Vec<Value>>(&raw.data)?;

    assert_eq!(typed.data.len(), 1);
    assert_eq!(raw_items.len(), 1);
    assert_eq!(
        raw_items[0]
            .get("message")
            .or_else(|| raw_items[0]
                .get("error")
                .and_then(|value| value.get("message")))
            .and_then(Value::as_str),
        typed.data[0]
            .error
            .as_ref()
            .and_then(|value| value.message.as_deref())
    );

    Ok(())
}
