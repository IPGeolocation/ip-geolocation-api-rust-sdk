use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl StdError for ValidationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiError {
    status_code: u16,
    message: String,
}

impl ApiError {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
        }
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} (status {})", self.message, self.status_code)
    }
}

impl StdError for ApiError {}

#[derive(Debug)]
pub enum IpGeolocationError {
    Validation(ValidationError),
    Api(ApiError),
    RequestTimeout(Cow<'static, str>),
    Transport {
        message: Cow<'static, str>,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    Serialization {
        message: Cow<'static, str>,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    ClientClosed,
}

impl Display for IpGeolocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Api(error) => error.fmt(formatter),
            Self::RequestTimeout(message) => formatter.write_str(message),
            Self::Transport { message, .. } => formatter.write_str(message),
            Self::Serialization { message, .. } => formatter.write_str(message),
            Self::ClientClosed => formatter.write_str("client is closed"),
        }
    }
}

impl StdError for IpGeolocationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Api(error) => Some(error),
            Self::Transport { source, .. } | Self::Serialization { source, .. } => source
                .as_deref()
                .map(|error| error as &(dyn StdError + 'static)),
            Self::RequestTimeout(_) | Self::ClientClosed => None,
        }
    }
}

impl From<ValidationError> for IpGeolocationError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl From<ApiError> for IpGeolocationError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl IpGeolocationError {
    pub(crate) fn redact(self, api_key: Option<&str>) -> Self {
        if api_key.is_none() {
            return self;
        }
        let clean = |message: Cow<'static, str>| redact_key(message.into_owned(), api_key).into();
        let clean_source = |source: Option<Box<dyn StdError + Send + Sync>>| {
            source.map(|source| {
                let mut item: Option<&(dyn StdError + 'static)> = Some(source.as_ref());
                while let Some(error) = item {
                    for text in [error.to_string(), format!("{error:?}")] {
                        if redact_key(text.clone(), api_key) != text {
                            return redact_key(source.to_string(), api_key).into();
                        }
                    }
                    item = error.source();
                }
                source
            })
        };
        match self {
            Self::Transport { message, source } => Self::Transport {
                message: clean(message),
                source: clean_source(source),
            },
            Self::Serialization { message, source } => Self::Serialization {
                message: clean(message),
                source: clean_source(source),
            },
            Self::Validation(error) => Self::validation_message(redact_key(error.message, api_key)),
            Self::Api(error) => {
                ApiError::new(error.status_code, redact_key(error.message, api_key)).into()
            }
            Self::RequestTimeout(message) => Self::RequestTimeout(clean(message)),
            Self::ClientClosed => Self::ClientClosed,
        }
    }

    pub fn validation_message(message: impl Into<String>) -> Self {
        Self::Validation(ValidationError::new(message))
    }

    pub fn request_timeout(message: impl Into<Cow<'static, str>>) -> Self {
        Self::RequestTimeout(message.into())
    }

    pub fn transport(
        message: impl Into<Cow<'static, str>>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::Transport {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn transport_message(message: impl Into<Cow<'static, str>>) -> Self {
        Self::Transport {
            message: message.into(),
            source: None,
        }
    }

    pub fn serialization(
        message: impl Into<Cow<'static, str>>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::Serialization {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

pub(crate) fn redact_key(message: String, api_key: Option<&str>) -> String {
    let Some(key) = api_key.map(str::trim).filter(|key| !key.is_empty()) else {
        return message;
    };
    let mut message = message;
    if message.contains(key) {
        message = message.replace(key, "[REDACTED]");
    }
    if message.contains('\\') {
        let escaped = serde_json::to_string(key).expect("writing JSON to a string cannot fail");
        let escaped = &escaped[1..escaped.len() - 1];
        if escaped != key && message.contains(escaped) {
            message = message.replace(escaped, "[REDACTED]");
        }
        if message.contains("\\u") {
            return "[REDACTED]".into();
        }
    }
    if message.contains('%') || message.contains('+') {
        let decoded = percent_encoding::percent_decode_str(&message).decode_utf8_lossy();
        if decoded.contains(key)
            || (message.contains('+')
                && percent_encoding::percent_decode_str(&message.replace('+', " "))
                    .decode_utf8_lossy()
                    .contains(key))
        {
            return "[REDACTED]".into();
        }
    }
    message
}

#[cfg(test)]
mod tests {
    use super::redact_key;

    #[test]
    fn plain_and_url_encoded_keys_are_hidden() {
        let key = Some("  example +/  ");
        for message in [
            "bad example +/",
            "bad example+%2B%2f",
            "bad %65xample%20%2b%2F",
            "example +/ and example+%2b%2f",
        ] {
            let result = redact_key(message.into(), key);
            assert!(result.contains("[REDACTED]"));
            assert!(!result.contains("example") && !result.contains("xample"));
        }
        assert_eq!(redact_key("try again in 5%".into(), key), "try again in 5%");
        assert_eq!(redact_key("no key set".into(), None), "no key set");
    }
}
