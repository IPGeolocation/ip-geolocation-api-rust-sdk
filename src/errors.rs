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
