use std::fmt::{Debug, Formatter};

use crate::errors::ValidationError;
use reqwest::Url;

const DEFAULT_BASE_URL: &str = "https://api.ipgeolocation.io";
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

#[derive(Clone, Eq, PartialEq)]
pub struct IpGeolocationClientConfig {
    pub api_key: Option<String>,
    pub request_origin: Option<String>,
    pub base_url: String,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
}

impl Default for IpGeolocationClientConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            request_origin: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            connect_timeout_ms: DEFAULT_CONNECT_TIMEOUT_MS,
            request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
        }
    }
}

impl Debug for IpGeolocationClientConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IpGeolocationClientConfig")
            .field(
                "api_key",
                &self
                    .api_key
                    .as_ref()
                    .map(|_| "[REDACTED]")
                    .unwrap_or("[UNSET]"),
            )
            .field("request_origin", &self.request_origin)
            .field("base_url", &self.base_url)
            .field("connect_timeout_ms", &self.connect_timeout_ms)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .finish()
    }
}

impl IpGeolocationClientConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(api_key) = &self.api_key {
            validate_non_blank(api_key, "api_key")?;
            validate_no_crlf(api_key, "api_key")?;
        }

        if let Some(request_origin) = &self.request_origin {
            validate_non_blank(request_origin, "request_origin")?;
            validate_no_crlf(request_origin, "request_origin")?;
            validate_request_origin(request_origin)?;
        }

        validate_non_blank(&self.base_url, "base_url")?;
        validate_base_url(&self.base_url)?;

        if self.connect_timeout_ms == 0 {
            return Err(ValidationError::new(
                "connect_timeout_ms must be greater than zero",
            ));
        }

        if self.request_timeout_ms == 0 {
            return Err(ValidationError::new(
                "request_timeout_ms must be greater than zero",
            ));
        }

        Ok(())
    }
}

fn validate_non_blank(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new(format!(
            "{field_name} must not be blank"
        )));
    }

    Ok(())
}

fn validate_no_crlf(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if value.contains('\r') || value.contains('\n') {
        return Err(ValidationError::new(format!(
            "{field_name} must not contain CR or LF"
        )));
    }

    Ok(())
}

fn parse_http_url(value: &str, field_name: &str) -> Result<Url, ValidationError> {
    let trimmed = value.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"));

    if let Some(without_scheme) = without_scheme {
        if without_scheme.is_empty() || without_scheme.starts_with('/') {
            return Err(ValidationError::new(format!(
                "{field_name} must include a valid host"
            )));
        }
    }

    let url = Url::parse(trimmed).map_err(|_| {
        ValidationError::new(format!(
            "{field_name} must be an absolute http or https URL"
        ))
    })?;

    match url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(ValidationError::new(format!(
                "{field_name} must be an absolute http or https URL"
            )))
        }
    }

    if url
        .host_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(ValidationError::new(format!(
            "{field_name} must include a valid host"
        )));
    }

    Ok(url)
}

fn validate_base_url(value: &str) -> Result<(), ValidationError> {
    let url = parse_http_url(value, "base_url")?;

    if url.query().is_some() || url.fragment().is_some() {
        return Err(ValidationError::new(
            "base_url must not include params, query, or fragment",
        ));
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(ValidationError::new("base_url must not include userinfo"));
    }

    Ok(())
}

fn validate_request_origin(value: &str) -> Result<(), ValidationError> {
    let url = parse_http_url(value, "request_origin")?;

    if !url.username().is_empty() || url.password().is_some() {
        return Err(ValidationError::new(
            "request_origin must not include userinfo",
        ));
    }

    if url.query().is_some() || url.fragment().is_some() {
        return Err(ValidationError::new(
            "request_origin must not include params, query, or fragment",
        ));
    }

    if url.path() != "/" {
        return Err(ValidationError::new(
            "request_origin must not include a path",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::IpGeolocationClientConfig;

    #[test]
    fn debug_redacts_api_key() {
        let config = IpGeolocationClientConfig {
            api_key: Some("secret".to_string()),
            ..IpGeolocationClientConfig::default()
        };

        let debug_output = format!("{config:?}");
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("secret"));
    }

    #[test]
    fn request_origin_rejects_paths() {
        let config = IpGeolocationClientConfig {
            request_origin: Some("https://app.example.com/path".to_string()),
            ..IpGeolocationClientConfig::default()
        };

        let error = config.validate().unwrap_err();
        assert_eq!(error.message(), "request_origin must not include a path");
    }

    #[test]
    fn invalid_base_url_is_rejected() {
        let config = IpGeolocationClientConfig {
            base_url: "https:///broken".to_string(),
            ..IpGeolocationClientConfig::default()
        };

        let error = config.validate().unwrap_err();
        assert_eq!(error.message(), "base_url must include a valid host");
    }

    #[test]
    fn invalid_request_origin_is_rejected() {
        let config = IpGeolocationClientConfig {
            request_origin: Some("https:///broken".to_string()),
            ..IpGeolocationClientConfig::default()
        };

        let error = config.validate().unwrap_err();
        assert_eq!(error.message(), "request_origin must include a valid host");
    }
}
