use std::collections::BTreeMap;

use crate::errors::ValidationError;
use crate::response_format::ResponseFormat;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LookupIpGeolocationRequest {
    pub ip: Option<String>,
    pub lang: Option<String>,
    pub include: Vec<String>,
    pub fields: Vec<String>,
    pub excludes: Vec<String>,
    pub user_agent: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub output: ResponseFormat,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BulkLookupIpGeolocationRequest {
    pub ips: Vec<String>,
    pub lang: Option<String>,
    pub include: Vec<String>,
    pub fields: Vec<String>,
    pub excludes: Vec<String>,
    pub user_agent: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub output: ResponseFormat,
}

impl LookupIpGeolocationRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(ip) = &self.ip {
            validate_lookup_value(ip, "ip")?;
        }

        validate_optional_language(self.lang.as_deref())?;
        validate_string_list(&self.include, "include", true)?;
        validate_string_list(&self.fields, "fields", true)?;
        validate_string_list(&self.excludes, "excludes", true)?;

        if let Some(user_agent) = &self.user_agent {
            validate_lookup_value(user_agent, "user_agent")?;
        }

        validate_headers(&self.headers)?;

        Ok(())
    }
}

impl BulkLookupIpGeolocationRequest {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.ips.is_empty() {
            return Err(ValidationError::new(
                "ips must contain at least one IP address or domain",
            ));
        }

        if self.ips.len() > 50_000 {
            return Err(ValidationError::new(
                "ips must not contain more than 50000 entries",
            ));
        }

        for ip in &self.ips {
            validate_lookup_value(ip, "ips")?;
        }

        validate_optional_language(self.lang.as_deref())?;
        validate_string_list(&self.include, "include", true)?;
        validate_string_list(&self.fields, "fields", true)?;
        validate_string_list(&self.excludes, "excludes", true)?;

        if let Some(user_agent) = &self.user_agent {
            validate_lookup_value(user_agent, "user_agent")?;
        }

        validate_headers(&self.headers)?;

        Ok(())
    }
}

fn validate_optional_language(value: Option<&str>) -> Result<(), ValidationError> {
    let Some(value) = value else {
        return Ok(());
    };

    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(ValidationError::new("lang must not be blank"));
    }

    match normalized.as_str() {
        "en" | "de" | "ru" | "ja" | "fr" | "cn" | "es" | "cs" | "it" | "ko" | "fa" | "pt" => Ok(()),
        _ => Err(ValidationError::new(
            "lang must be one of: en, de, ru, ja, fr, cn, es, cs, it, ko, fa, pt",
        )),
    }
}

fn validate_string_list(
    values: &[String],
    field_name: &str,
    dedupe: bool,
) -> Result<(), ValidationError> {
    let mut seen = Vec::<&str>::new();
    for value in values {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(ValidationError::new(format!(
                "{field_name} must not contain blank values"
            )));
        }

        if dedupe && seen.contains(&normalized) {
            continue;
        }

        seen.push(normalized);
    }

    Ok(())
}

fn validate_headers(headers: &BTreeMap<String, String>) -> Result<(), ValidationError> {
    for (name, value) in headers {
        if name.trim().is_empty() {
            return Err(ValidationError::new("headers must not contain blank names"));
        }

        if value.trim().is_empty() {
            return Err(ValidationError::new(
                "headers must not contain blank values",
            ));
        }

        if name.contains('\r')
            || name.contains('\n')
            || value.contains('\r')
            || value.contains('\n')
        {
            return Err(ValidationError::new("headers must not contain CR or LF"));
        }
    }

    Ok(())
}

fn validate_lookup_value(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new(format!(
            "{field_name} must not be blank"
        )));
    }

    if value.contains('\r') || value.contains('\n') {
        return Err(ValidationError::new(format!(
            "{field_name} must not contain CR or LF"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BulkLookupIpGeolocationRequest, LookupIpGeolocationRequest};

    #[test]
    fn bulk_lookup_preserves_duplicates() {
        let request = BulkLookupIpGeolocationRequest {
            ips: vec![
                "8.8.8.8".to_string(),
                "8.8.8.8".to_string(),
                "ipgeolocation.io".to_string(),
            ],
            ..BulkLookupIpGeolocationRequest::default()
        };

        assert!(request.validate().is_ok());
        assert_eq!(
            request.ips,
            vec![
                "8.8.8.8".to_string(),
                "8.8.8.8".to_string(),
                "ipgeolocation.io".to_string()
            ]
        );
    }

    #[test]
    fn single_lookup_rejects_blank_header_names() {
        let mut request = LookupIpGeolocationRequest::default();
        request
            .headers
            .insert("   ".to_string(), "value".to_string());

        let error = request.validate().unwrap_err();
        assert_eq!(error.message(), "headers must not contain blank names");
    }
}
