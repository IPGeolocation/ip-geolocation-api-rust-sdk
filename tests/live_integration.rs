use ipgeolocation::{
    BulkLookupIpGeolocationRequest, IpGeolocationClient, IpGeolocationClientConfig,
    LookupIpGeolocationRequest,
};

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

#[test]
fn live_single_lookup_with_api_key() -> Result<(), Box<dyn std::error::Error>> {
    if !env_flag("IPGEO_RUN_LIVE_TESTS") {
        return Ok(());
    }

    let Some(api_key) = env_value("IPGEO_FREE_KEY").or_else(|| env_value("IPGEO_PAID_KEY")) else {
        return Ok(());
    };

    let client = IpGeolocationClient::new(IpGeolocationClientConfig {
        api_key: Some(api_key),
        ..IpGeolocationClientConfig::default()
    })?;

    let response = client.lookup_ip_geolocation(&LookupIpGeolocationRequest {
        ip: Some("8.8.8.8".to_string()),
        ..LookupIpGeolocationRequest::default()
    })?;

    assert_eq!(response.metadata.status_code, 200);
    assert_eq!(response.data.ip.as_deref(), Some("8.8.8.8"));
    assert!(response.metadata.credits_charged.unwrap_or(0) >= 1);

    Ok(())
}

#[test]
fn live_bulk_lookup_with_paid_key() -> Result<(), Box<dyn std::error::Error>> {
    if !env_flag("IPGEO_RUN_LIVE_TESTS") {
        return Ok(());
    }

    let Some(api_key) = env_value("IPGEO_PAID_KEY") else {
        return Ok(());
    };

    let client = IpGeolocationClient::new(IpGeolocationClientConfig {
        api_key: Some(api_key),
        ..IpGeolocationClientConfig::default()
    })?;

    let response = client.bulk_lookup_ip_geolocation(&BulkLookupIpGeolocationRequest {
        ips: vec!["8.8.8.8".to_string(), "invalid-ip".to_string()],
        ..BulkLookupIpGeolocationRequest::default()
    })?;

    assert_eq!(response.metadata.status_code, 200);
    assert_eq!(response.data.len(), 2);
    assert_eq!(
        response.data[0]
            .data
            .as_ref()
            .and_then(|value| value.ip.as_deref()),
        Some("8.8.8.8")
    );
    assert!(response.data[1]
        .error
        .as_ref()
        .and_then(|value| value.message.as_deref())
        .is_some());

    Ok(())
}
