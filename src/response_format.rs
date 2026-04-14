#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseFormat {
    Json,
    Xml,
}

impl Default for ResponseFormat {
    fn default() -> Self {
        Self::Json
    }
}

impl ResponseFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Xml => "xml",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseFormat;

    #[test]
    fn response_format_strings_match_wire_values() {
        assert_eq!(ResponseFormat::Json.as_str(), "json");
        assert_eq!(ResponseFormat::Xml.as_str(), "xml");
    }
}
