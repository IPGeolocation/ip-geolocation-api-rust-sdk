use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, Url};
use serde::Serialize;
use serde_json::Value;

use crate::api_response::{ApiResponse, ApiResponseMetadata};
use crate::config::IpGeolocationClientConfig;
use crate::errors::{ApiError, IpGeolocationError, ValidationError};
use crate::models::{BulkLookupError, BulkLookupResult, IpGeolocationResponse};
use crate::requests::{BulkLookupIpGeolocationRequest, LookupIpGeolocationRequest};
use crate::response_format::ResponseFormat;
use crate::transport::{HttpRequestData, HttpTransport, ReqwestBlockingTransport};
use crate::version::VERSION;

#[derive(Clone)]
pub struct IpGeolocationClient {
    config: IpGeolocationClientConfig,
    transport: Arc<dyn HttpTransport>,
    closed: bool,
}

impl Debug for IpGeolocationClient {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IpGeolocationClient")
            .field("config", &self.config)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl IpGeolocationClient {
    /// Main client for the IPGeolocation.io IP Location API.
    ///
    /// Homepage: <https://ipgeolocation.io>
    ///
    /// IP Location API: <https://ipgeolocation.io/ip-location-api.html>
    ///
    /// Documentation: <https://ipgeolocation.io/documentation/ip-location-api.html>
    pub fn new(config: IpGeolocationClientConfig) -> Result<Self, IpGeolocationError> {
        config.validate()?;

        let transport =
            ReqwestBlockingTransport::new(Duration::from_millis(config.connect_timeout_ms))?;

        Ok(Self {
            config,
            transport: Arc::new(transport),
            closed: false,
        })
    }

    pub fn default_user_agent() -> String {
        format!("ipgeolocation-rust-sdk/{VERSION}")
    }

    pub fn lookup_ip_geolocation(
        &self,
        request: &LookupIpGeolocationRequest,
    ) -> Result<ApiResponse<IpGeolocationResponse>, IpGeolocationError> {
        self.ensure_open()?;
        self.validate_single_request(request)?;

        let request_data = self.build_lookup_request(request)?;
        let (response, duration_ms) = self.transport.send(&request_data)?;

        if !is_success_status(response.status_code) {
            return Err(to_api_error(response.status_code, &response.body));
        }

        let data =
            serde_json::from_str::<IpGeolocationResponse>(&response.body).map_err(|error| {
                IpGeolocationError::serialization(
                    "failed to parse single lookup response body",
                    error,
                )
            })?;

        Ok(ApiResponse {
            data,
            metadata: to_metadata(response.status_code, duration_ms, response.headers),
        })
    }

    pub fn lookup_ip_geolocation_raw(
        &self,
        request: &LookupIpGeolocationRequest,
    ) -> Result<ApiResponse<String>, IpGeolocationError> {
        self.ensure_open()?;
        self.validate_single_raw_request(request)?;

        let request_data = self.build_lookup_request(request)?;
        let (response, duration_ms) = self.transport.send(&request_data)?;

        if !is_success_status(response.status_code) {
            return Err(to_api_error(response.status_code, &response.body));
        }

        Ok(ApiResponse {
            data: response.body,
            metadata: to_metadata(response.status_code, duration_ms, response.headers),
        })
    }

    pub fn bulk_lookup_ip_geolocation(
        &self,
        request: &BulkLookupIpGeolocationRequest,
    ) -> Result<ApiResponse<Vec<BulkLookupResult>>, IpGeolocationError> {
        self.ensure_open()?;
        self.validate_bulk_request(request)?;

        let request_data = self.build_bulk_request(request)?;
        let (response, duration_ms) = self.transport.send(&request_data)?;

        if !is_success_status(response.status_code) {
            return Err(to_api_error(response.status_code, &response.body));
        }

        let data = parse_bulk_lookup_results(&response.body)?;

        Ok(ApiResponse {
            data,
            metadata: to_metadata(response.status_code, duration_ms, response.headers),
        })
    }

    pub fn bulk_lookup_ip_geolocation_raw(
        &self,
        request: &BulkLookupIpGeolocationRequest,
    ) -> Result<ApiResponse<String>, IpGeolocationError> {
        self.ensure_open()?;
        self.validate_bulk_raw_request(request)?;

        let request_data = self.build_bulk_request(request)?;
        let (response, duration_ms) = self.transport.send(&request_data)?;

        if !is_success_status(response.status_code) {
            return Err(to_api_error(response.status_code, &response.body));
        }

        Ok(ApiResponse {
            data: response.body,
            metadata: to_metadata(response.status_code, duration_ms, response.headers),
        })
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    #[cfg(test)]
    pub(crate) fn with_transport(
        config: IpGeolocationClientConfig,
        transport: Arc<dyn HttpTransport>,
    ) -> Result<Self, IpGeolocationError> {
        config.validate()?;

        Ok(Self {
            config,
            transport,
            closed: false,
        })
    }

    fn ensure_open(&self) -> Result<(), IpGeolocationError> {
        if self.closed {
            return Err(IpGeolocationError::ClientClosed);
        }

        Ok(())
    }

    fn validate_single_request(
        &self,
        request: &LookupIpGeolocationRequest,
    ) -> Result<(), IpGeolocationError> {
        if self.config.api_key.is_none() && self.config.request_origin.is_none() {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "single lookup requires api_key or request_origin in client config",
            )));
        }

        request.validate()?;

        if request.output == ResponseFormat::Xml {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "typed methods support JSON only",
            )));
        }

        Ok(())
    }

    fn validate_single_raw_request(
        &self,
        request: &LookupIpGeolocationRequest,
    ) -> Result<(), IpGeolocationError> {
        if self.config.api_key.is_none() && self.config.request_origin.is_none() {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "single lookup requires api_key or request_origin in client config",
            )));
        }

        request.validate()?;
        Ok(())
    }

    fn validate_bulk_request(
        &self,
        request: &BulkLookupIpGeolocationRequest,
    ) -> Result<(), IpGeolocationError> {
        if self.config.api_key.is_none() {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "bulk lookup requires api_key in client config",
            )));
        }

        request.validate()?;

        if request.output == ResponseFormat::Xml {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "typed methods support JSON only",
            )));
        }

        Ok(())
    }

    fn validate_bulk_raw_request(
        &self,
        request: &BulkLookupIpGeolocationRequest,
    ) -> Result<(), IpGeolocationError> {
        if self.config.api_key.is_none() {
            return Err(IpGeolocationError::Validation(ValidationError::new(
                "bulk lookup requires api_key in client config",
            )));
        }

        request.validate()?;
        Ok(())
    }

    fn build_lookup_request(
        &self,
        request: &LookupIpGeolocationRequest,
    ) -> Result<HttpRequestData, IpGeolocationError> {
        let mut url = parse_base_url(&self.config.base_url, "/v3/ipgeo")?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(api_key) = normalize_optional_string(self.config.api_key.as_deref()) {
                query.append_pair("apiKey", &api_key);
            }
            if let Some(ip) = normalize_optional_string(request.ip.as_deref()) {
                query.append_pair("ip", &ip);
            }
            if let Some(lang) = normalize_optional_string(request.lang.as_deref()) {
                query.append_pair("lang", &lang);
            }
            if let Some(include) = join_trimmed(&request.include) {
                query.append_pair("include", &include);
            }
            if let Some(fields) = join_trimmed(&request.fields) {
                query.append_pair("fields", &fields);
            }
            if let Some(excludes) = join_trimmed(&request.excludes) {
                query.append_pair("excludes", &excludes);
            }
            query.append_pair("output", request.output.as_str());
        }

        Ok(HttpRequestData {
            method: Method::GET,
            url,
            headers: self.build_headers(
                &request.headers,
                request.user_agent.as_deref(),
                request.output,
                false,
            ),
            body: None,
            request_timeout: Duration::from_millis(self.config.request_timeout_ms),
        })
    }

    fn build_bulk_request(
        &self,
        request: &BulkLookupIpGeolocationRequest,
    ) -> Result<HttpRequestData, IpGeolocationError> {
        let mut url = parse_base_url(&self.config.base_url, "/v3/ipgeo-bulk")?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(api_key) = normalize_optional_string(self.config.api_key.as_deref()) {
                query.append_pair("apiKey", &api_key);
            }
            if let Some(lang) = normalize_optional_string(request.lang.as_deref()) {
                query.append_pair("lang", &lang);
            }
            if let Some(include) = join_trimmed(&request.include) {
                query.append_pair("include", &include);
            }
            if let Some(fields) = join_trimmed(&request.fields) {
                query.append_pair("fields", &fields);
            }
            if let Some(excludes) = join_trimmed(&request.excludes) {
                query.append_pair("excludes", &excludes);
            }
            query.append_pair("output", request.output.as_str());
        }

        let ips = request
            .ips
            .iter()
            .map(|value| value.trim().to_string())
            .collect::<Vec<_>>();
        let body = serde_json::to_vec(&BulkLookupRequestBody { ips: &ips }).map_err(|error| {
            IpGeolocationError::serialization("failed to serialize bulk lookup request body", error)
        })?;

        Ok(HttpRequestData {
            method: Method::POST,
            url,
            headers: self.build_headers(
                &request.headers,
                request.user_agent.as_deref(),
                request.output,
                true,
            ),
            body: Some(body),
            request_timeout: Duration::from_millis(self.config.request_timeout_ms),
        })
    }

    fn build_headers(
        &self,
        request_headers: &BTreeMap<String, String>,
        request_user_agent: Option<&str>,
        output: ResponseFormat,
        include_content_type: bool,
    ) -> BTreeMap<String, String> {
        let mut headers = BTreeMap::<String, String>::new();
        for (name, value) in request_headers {
            headers.insert(name.trim().to_string(), value.trim().to_string());
        }

        headers.insert(
            "User-Agent".to_string(),
            resolve_user_agent(request_user_agent, request_headers),
        );
        headers.insert(
            "Accept".to_string(),
            if output == ResponseFormat::Xml {
                "application/xml".to_string()
            } else {
                "application/json".to_string()
            },
        );

        if let Some(request_origin) =
            normalize_optional_string(self.config.request_origin.as_deref())
        {
            headers.insert("Origin".to_string(), request_origin);
        }

        if include_content_type {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }

        headers
    }
}

#[derive(Serialize)]
struct BulkLookupRequestBody<'a> {
    ips: &'a [String],
}

fn parse_base_url(base_url: &str, path: &str) -> Result<Url, IpGeolocationError> {
    let normalized = base_url.trim_end_matches('/');
    Url::parse(&format!("{normalized}{path}")).map_err(|error| {
        IpGeolocationError::validation_message(format!(
            "invalid base_url after normalization: {error}"
        ))
    })
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn join_trimmed(values: &[String]) -> Option<String> {
    let joined = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(",");

    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn resolve_user_agent(
    request_user_agent: Option<&str>,
    headers: &BTreeMap<String, String>,
) -> String {
    if let Some(user_agent) = normalize_optional_string(request_user_agent) {
        return user_agent;
    }

    for (name, value) in headers {
        if name.eq_ignore_ascii_case("User-Agent") {
            if let Some(user_agent) = normalize_optional_string(Some(value.as_str())) {
                return user_agent;
            }
        }
    }

    IpGeolocationClient::default_user_agent()
}

fn is_success_status(status_code: u16) -> bool {
    (200..=299).contains(&status_code)
}

fn to_metadata(
    status_code: u16,
    duration_ms: u64,
    headers: BTreeMap<String, Vec<String>>,
) -> ApiResponseMetadata {
    ApiResponseMetadata {
        credits_charged: parse_u32_header(first_header_ignore_case(&headers, "X-Credits-Charged")),
        successful_records: parse_u32_header(
            first_header_ignore_case(&headers, "X-Successful-Record")
                .or_else(|| first_header_ignore_case(&headers, "X-Successful-Records")),
        ),
        status_code,
        duration_ms,
        raw_headers: headers,
    }
}

fn first_header_ignore_case<'a>(
    headers: &'a BTreeMap<String, Vec<String>>,
    name: &str,
) -> Option<&'a str> {
    let expected = name.to_ascii_lowercase();
    headers.iter().find_map(|(key, values)| {
        if key.to_ascii_lowercase() == expected {
            values.first().map(String::as_str)
        } else {
            None
        }
    })
}

fn parse_u32_header(value: Option<&str>) -> Option<u32> {
    value.and_then(|value| value.parse::<u32>().ok())
}

fn to_api_error(status_code: u16, body: &str) -> IpGeolocationError {
    let message =
        extract_api_message(body).unwrap_or_else(|| default_api_error_message(status_code));
    ApiError::new(status_code, message).into()
}

fn extract_api_message(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    let value = match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => value,
        Err(_) => return Some(trimmed.to_string()),
    };
    if let Some(message) = value.get("message").and_then(Value::as_str) {
        return Some(message.to_string());
    }

    if let Some(message) = value
        .get("error")
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
    {
        return Some(message.to_string());
    }

    Some(trimmed.to_string())
}

fn default_api_error_message(status_code: u16) -> String {
    match status_code {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        423 => "Locked",
        429 => "Too Many Requests",
        499 => "Client Closed Request",
        500 => "Internal Server Error",
        _ => "API request failed",
    }
    .to_string()
}

fn parse_bulk_lookup_results(body: &str) -> Result<Vec<BulkLookupResult>, IpGeolocationError> {
    let values = serde_json::from_str::<Vec<Value>>(body).map_err(|error| {
        IpGeolocationError::serialization("failed to parse bulk lookup response body", error)
    })?;

    let mut results = Vec::with_capacity(values.len());
    for value in values {
        if is_bulk_error_value(&value) {
            results.push(BulkLookupResult {
                data: None,
                error: Some(BulkLookupError {
                    message: Some(read_bulk_error_message(&value)),
                }),
            });
            continue;
        }

        let response = serde_json::from_value::<IpGeolocationResponse>(value).map_err(|error| {
            IpGeolocationError::serialization("failed to parse bulk lookup response item", error)
        })?;
        results.push(BulkLookupResult {
            data: Some(response),
            error: None,
        });
    }

    Ok(results)
}

fn is_bulk_error_value(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    object.contains_key("error") || (object.contains_key("message") && !object.contains_key("ip"))
}

fn read_bulk_error_message(value: &Value) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Bulk lookup item failed".to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::transport::HttpResponseData;

    #[derive(Clone)]
    struct MockTransport {
        requests: Arc<Mutex<Vec<HttpRequestData>>>,
        response: HttpResponseData,
        duration_ms: u64,
    }

    impl MockTransport {
        fn new(response: HttpResponseData, duration_ms: u64) -> Self {
            Self {
                requests: Arc::new(Mutex::new(Vec::new())),
                response,
                duration_ms,
            }
        }

        fn recorded_requests(&self) -> Vec<HttpRequestData> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl HttpTransport for MockTransport {
        fn send(
            &self,
            request: &HttpRequestData,
        ) -> Result<(HttpResponseData, u64), IpGeolocationError> {
            self.requests.lock().unwrap().push(request.clone());
            Ok((self.response.clone(), self.duration_ms))
        }
    }

    struct ErrorTransport;

    impl HttpTransport for ErrorTransport {
        fn send(
            &self,
            _request: &HttpRequestData,
        ) -> Result<(HttpResponseData, u64), IpGeolocationError> {
            Err(IpGeolocationError::transport_message("transport failed"))
        }
    }

    #[test]
    fn default_user_agent_contains_version() {
        assert!(IpGeolocationClient::default_user_agent().starts_with("ipgeolocation-rust-sdk/"));
    }

    #[test]
    fn invalid_base_url_prevents_client_construction() {
        let error = IpGeolocationClient::new(IpGeolocationClientConfig {
            base_url: "https:///broken".to_string(),
            ..IpGeolocationClientConfig::default()
        })
        .unwrap_err();

        assert_eq!(error.to_string(), "base_url must include a valid host");
    }

    #[test]
    fn invalid_request_origin_prevents_client_construction() {
        let error = IpGeolocationClient::new(IpGeolocationClientConfig {
            request_origin: Some("https:///broken".to_string()),
            ..IpGeolocationClientConfig::default()
        })
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "request_origin must include a valid host"
        );
    }

    #[test]
    fn single_lookup_requires_auth() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig::default(),
            Arc::new(MockTransport::new(HttpResponseData::default(), 0)),
        )
        .unwrap();

        let error = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "single lookup requires api_key or request_origin in client config"
        );
    }

    #[test]
    fn closed_client_rejects_requests() {
        let mut client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(MockTransport::new(HttpResponseData::default(), 0)),
        )
        .unwrap();

        client.close();

        let error = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap_err();

        assert_eq!(error.to_string(), "client is closed");
    }

    #[test]
    fn bulk_lookup_requires_api_key() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig::default(),
            Arc::new(MockTransport::new(HttpResponseData::default(), 0)),
        )
        .unwrap();
        let request = BulkLookupIpGeolocationRequest {
            ips: vec!["8.8.8.8".to_string()],
            ..BulkLookupIpGeolocationRequest::default()
        };

        let error = client.bulk_lookup_ip_geolocation(&request).unwrap_err();
        assert_eq!(
            error.to_string(),
            "bulk lookup requires api_key in client config"
        );
    }

    #[test]
    fn typed_methods_reject_xml() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(MockTransport::new(HttpResponseData::default(), 0)),
        )
        .unwrap();

        let error = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest {
                output: ResponseFormat::Xml,
                ..LookupIpGeolocationRequest::default()
            })
            .unwrap_err();

        assert_eq!(error.to_string(), "typed methods support JSON only");
    }

    #[test]
    fn raw_single_lookup_allows_xml_and_sets_headers() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: "<response/>".to_string(),
                headers: BTreeMap::new(),
            },
            7,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                request_origin: Some("https://app.example.com".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport.clone()),
        )
        .unwrap();

        let response = client
            .lookup_ip_geolocation_raw(&LookupIpGeolocationRequest {
                ip: Some(" 8.8.8.8 ".to_string()),
                output: ResponseFormat::Xml,
                ..LookupIpGeolocationRequest::default()
            })
            .unwrap();

        assert_eq!(response.data, "<response/>");
        let requests = transport.recorded_requests();
        let request = requests.first().unwrap();
        assert_eq!(request.headers.get("Accept").unwrap(), "application/xml");
        assert_eq!(
            request.headers.get("Origin").unwrap(),
            "https://app.example.com"
        );
        assert!(request.url.as_str().contains("ip=8.8.8.8"));
    }

    #[test]
    fn raw_bulk_lookup_allows_xml_and_sets_request_headers() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: "<response/>".to_string(),
                headers: BTreeMap::new(),
            },
            6,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport.clone()),
        )
        .unwrap();

        let response = client
            .bulk_lookup_ip_geolocation_raw(&BulkLookupIpGeolocationRequest {
                ips: vec!["8.8.8.8".to_string()],
                output: ResponseFormat::Xml,
                ..BulkLookupIpGeolocationRequest::default()
            })
            .unwrap();

        assert_eq!(response.data, "<response/>");
        let requests = transport.recorded_requests();
        let request = requests.first().unwrap();
        assert_eq!(request.headers.get("Accept").unwrap(), "application/xml");
        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn lookup_parses_single_response_and_metadata() {
        let mut headers = BTreeMap::new();
        headers.insert("X-Credits-Charged".to_string(), vec!["2".to_string()]);
        headers.insert("X-Successful-Record".to_string(), vec!["1".to_string()]);

        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: serde_json::json!({
                    "ip": "8.8.8.8",
                    "hostname": "dns.google",
                    "asn": { "as_number": "AS15169" },
                    "company": { "name": "Google LLC" },
                    "time_zone": {
                        "name": "America/Chicago",
                        "current_tz_abbreviation": "CDT"
                    }
                })
                .to_string(),
                headers,
            },
            15,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let response = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap();

        assert_eq!(response.data.ip.as_deref(), Some("8.8.8.8"));
        assert_eq!(response.data.hostname.as_deref(), Some("dns.google"));
        assert_eq!(
            response
                .data
                .asn
                .as_ref()
                .and_then(|value| value.as_number.as_deref()),
            Some("AS15169")
        );
        assert_eq!(
            response
                .data
                .time_zone
                .as_ref()
                .and_then(|value| value.current_tz_abbreviation.as_deref()),
            Some("CDT")
        );
        assert_eq!(response.metadata.credits_charged, Some(2));
        assert_eq!(response.metadata.successful_records, Some(1));
        assert_eq!(response.metadata.duration_ms, 15);
    }

    #[test]
    fn metadata_ignores_invalid_numbers_and_matches_headers_case_insensitively() {
        let mut headers = BTreeMap::new();
        headers.insert(
            "x-credits-charged".to_string(),
            vec!["not-a-number".to_string()],
        );
        headers.insert("X-SUCCESSFUL-RECORDS".to_string(), vec!["4".to_string()]);

        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: serde_json::json!({ "ip": "8.8.8.8" }).to_string(),
                headers: headers.clone(),
            },
            11,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let response = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap();

        assert_eq!(response.metadata.credits_charged, None);
        assert_eq!(response.metadata.successful_records, Some(4));
        assert_eq!(response.metadata.raw_headers, headers);
    }

    #[test]
    fn lookup_parses_legacy_time_zone_aliases() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: serde_json::json!({
                    "ip": "8.8.8.8",
                    "time_zone": {
                        "current_timezone_abbreviation": "CDT",
                        "current_timezone_name": "Central Daylight Time",
                        "timezone_abbreviation": "CST",
                        "timezone_name": "Central Standard Time",
                        "dst_timezone_abbreviation": "CDT",
                        "dst_timezone_name": "Central Daylight Time"
                    }
                })
                .to_string(),
                headers: BTreeMap::new(),
            },
            8,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let response = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap();

        let time_zone = response.data.time_zone.unwrap();
        assert_eq!(time_zone.current_tz_abbreviation.as_deref(), Some("CDT"));
        assert_eq!(
            time_zone.current_tz_full_name.as_deref(),
            Some("Central Daylight Time")
        );
        assert_eq!(time_zone.standard_tz_abbreviation.as_deref(), Some("CST"));
        assert_eq!(
            time_zone.standard_tz_full_name.as_deref(),
            Some("Central Standard Time")
        );
        assert_eq!(time_zone.dst_tz_abbreviation.as_deref(), Some("CDT"));
        assert_eq!(
            time_zone.dst_tz_full_name.as_deref(),
            Some("Central Daylight Time")
        );
    }

    #[test]
    fn omitted_list_fields_stay_omitted() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: serde_json::json!({
                    "ip": "8.8.8.8",
                    "country_metadata": {
                        "calling_code": "+1"
                    },
                    "security": {
                        "is_proxy": false
                    },
                    "abuse": {
                        "name": "Example Abuse Desk"
                    }
                })
                .to_string(),
                headers: BTreeMap::new(),
            },
            8,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let response = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap();

        assert_eq!(
            response
                .data
                .country_metadata
                .as_ref()
                .and_then(|value| value.languages.as_ref()),
            None
        );
        assert_eq!(
            response
                .data
                .security
                .as_ref()
                .and_then(|value| value.proxy_provider_names.as_ref()),
            None
        );
        assert_eq!(
            response
                .data
                .security
                .as_ref()
                .and_then(|value| value.vpn_provider_names.as_ref()),
            None
        );
        assert_eq!(
            response
                .data
                .abuse
                .as_ref()
                .and_then(|value| value.emails.as_ref()),
            None
        );
        assert_eq!(
            response
                .data
                .abuse
                .as_ref()
                .and_then(|value| value.phone_numbers.as_ref()),
            None
        );
    }

    #[test]
    fn bulk_lookup_preserves_duplicates_and_order() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: "[]".to_string(),
                headers: BTreeMap::new(),
            },
            4,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport.clone()),
        )
        .unwrap();

        client
            .bulk_lookup_ip_geolocation_raw(&BulkLookupIpGeolocationRequest {
                ips: vec![
                    " 8.8.8.8 ".to_string(),
                    "8.8.8.8".to_string(),
                    " ipgeolocation.io ".to_string(),
                ],
                ..BulkLookupIpGeolocationRequest::default()
            })
            .unwrap();

        let requests = transport.recorded_requests();
        let request = requests.first().unwrap();
        let body = String::from_utf8(request.body.clone().unwrap()).unwrap();
        assert_eq!(body, r#"{"ips":["8.8.8.8","8.8.8.8","ipgeolocation.io"]}"#);
    }

    #[test]
    fn bulk_lookup_parses_mixed_success_and_error_results() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 200,
                body: serde_json::json!([
                    {
                        "ip": "8.8.8.8",
                        "company": { "name": "Google LLC" }
                    },
                    {
                        "error": { "message": "invalid-ip is not a valid IP address." }
                    }
                ])
                .to_string(),
                headers: BTreeMap::new(),
            },
            5,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let response = client
            .bulk_lookup_ip_geolocation(&BulkLookupIpGeolocationRequest {
                ips: vec!["8.8.8.8".to_string(), "invalid-ip".to_string()],
                ..BulkLookupIpGeolocationRequest::default()
            })
            .unwrap();

        assert!(response.data[0].is_success());
        assert_eq!(
            response.data[0]
                .data
                .as_ref()
                .and_then(|value| value.company.as_ref())
                .and_then(|value| value.name.as_deref()),
            Some("Google LLC")
        );
        assert_eq!(
            response.data[1]
                .error
                .as_ref()
                .and_then(|value| value.message.as_deref()),
            Some("invalid-ip is not a valid IP address.")
        );
    }

    #[test]
    fn invalid_single_json_returns_serialization_error() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(MockTransport::new(
                HttpResponseData {
                    status_code: 200,
                    body: "{not-json}".to_string(),
                    headers: BTreeMap::new(),
                },
                2,
            )),
        )
        .unwrap();

        let error = client
            .lookup_ip_geolocation(&LookupIpGeolocationRequest::default())
            .unwrap_err();

        assert!(matches!(error, IpGeolocationError::Serialization { .. }));
        assert_eq!(
            error.to_string(),
            "failed to parse single lookup response body"
        );
    }

    #[test]
    fn invalid_bulk_json_returns_serialization_error() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(MockTransport::new(
                HttpResponseData {
                    status_code: 200,
                    body: "{not-json}".to_string(),
                    headers: BTreeMap::new(),
                },
                2,
            )),
        )
        .unwrap();

        let error = client
            .bulk_lookup_ip_geolocation(&BulkLookupIpGeolocationRequest {
                ips: vec!["8.8.8.8".to_string()],
                ..BulkLookupIpGeolocationRequest::default()
            })
            .unwrap_err();

        assert!(matches!(error, IpGeolocationError::Serialization { .. }));
        assert_eq!(
            error.to_string(),
            "failed to parse bulk lookup response body"
        );
    }

    #[test]
    fn transport_errors_are_returned() {
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(ErrorTransport),
        )
        .unwrap();

        let error = client
            .lookup_ip_geolocation_raw(&LookupIpGeolocationRequest::default())
            .unwrap_err();

        assert_eq!(error.to_string(), "transport failed");
    }

    #[test]
    fn api_errors_use_nested_error_message() {
        let transport = MockTransport::new(
            HttpResponseData {
                status_code: 400,
                body: serde_json::json!({
                    "error": { "message": "Invalid IP address" }
                })
                .to_string(),
                headers: BTreeMap::new(),
            },
            3,
        );
        let client = IpGeolocationClient::with_transport(
            IpGeolocationClientConfig {
                api_key: Some("secret".to_string()),
                ..IpGeolocationClientConfig::default()
            },
            Arc::new(transport),
        )
        .unwrap();

        let error = client
            .lookup_ip_geolocation_raw(&LookupIpGeolocationRequest::default())
            .unwrap_err();

        assert_eq!(error.to_string(), "Invalid IP address (status 400)");
    }
}
