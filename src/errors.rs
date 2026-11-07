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
    let message = if message.contains(key) {
        message.replace(key, "[REDACTED]")
    } else {
        message
    };
    if message.contains(['%', '\\', '&', '+'])
        && (has_encoded_key(&message, key, false)
            || (key.contains(' ') && has_encoded_key(&message, key, true)))
    {
        return "[REDACTED]".into();
    }
    message
}

fn has_encoded_key(message: &str, key: &str, form_spaces: bool) -> bool {
    let mut text = Cow::Borrowed(message);
    for _ in 0..4 {
        let mut unescaped = unescape_error_text(&text);
        if unescaped.contains(key) {
            return true;
        }
        if form_spaces && unescaped.contains('+') {
            unescaped = Cow::Owned(unescaped.replace('+', " "));
        }
        let decoded = percent_encoding::percent_decode_str(&unescaped).decode_utf8_lossy();
        if decoded.contains(key) {
            return true;
        }
        if decoded == text {
            return false;
        }
        text = Cow::Owned(decoded.into_owned());
    }
    text.contains(['%', '\\', '&'])
}

fn unescape_error_text(text: &str) -> Cow<'_, str> {
    let mut result = String::new();
    let mut copied = 0;
    for (i, _) in text.match_indices(['\\', '&']) {
        if i < copied {
            continue;
        }
        if let Some((len, ch)) = escaped_char(&text[i..]) {
            result.push_str(&text[copied..i]);
            result.push(ch);
            copied = i + len;
        }
    }
    if copied == 0 {
        return Cow::Borrowed(text);
    }
    result.push_str(&text[copied..]);
    Cow::Owned(result)
}

fn escaped_char(text: &str) -> Option<(usize, char)> {
    if text.starts_with('&') {
        let end = text.bytes().take(14).position(|b| b == b';')?;
        let ch = match &text[1..end] {
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            "lt" => '<',
            "gt" => '>',
            entity => {
                let digits = entity.strip_prefix('#')?;
                let value = match digits
                    .strip_prefix('x')
                    .or_else(|| digits.strip_prefix('X'))
                {
                    Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                    None => digits.parse().ok()?,
                };
                char::from_u32(value)?
            }
        };
        return Some((end + 1, ch));
    }
    let ch = match text.as_bytes().get(1)? {
        b'/' => '/',
        b'\\' => '\\',
        b'"' => '"',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'b' => '\x08',
        b'f' => '\x0c',
        b'u' => {
            let first = u16::from_str_radix(text.get(2..6)?, 16).ok()?;
            if let Some(ch) = char::from_u32(u32::from(first)) {
                return Some((6, ch));
            }
            if text.get(6..8) != Some("\\u") {
                return None;
            }
            let second = u16::from_str_radix(text.get(8..12)?, 16).ok()?;
            return Some((12, char::decode_utf16([first, second]).next()?.ok()?));
        }
        _ => return None,
    };
    Some((2, ch))
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
