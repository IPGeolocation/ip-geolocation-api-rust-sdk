use std::collections::BTreeMap;
use std::io::Read;
use std::time::{Duration, Instant};

use reqwest::blocking::Client as ReqwestClient;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Url};

use crate::errors::IpGeolocationError;

const MAX_RESPONSE_BODY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct HttpRequestData {
    pub method: Method,
    pub url: Url,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub request_timeout: Duration,
}

#[derive(Clone, Debug, Default)]
pub struct HttpResponseData {
    pub status_code: u16,
    pub body: String,
    pub headers: BTreeMap<String, Vec<String>>,
}

pub trait HttpTransport: Send + Sync {
    fn send(
        &self,
        request: &HttpRequestData,
    ) -> Result<(HttpResponseData, u64), IpGeolocationError>;
}

#[derive(Clone)]
pub struct ReqwestBlockingTransport {
    client: ReqwestClient,
}

impl ReqwestBlockingTransport {
    pub fn new(connect_timeout: Duration) -> Result<Self, IpGeolocationError> {
        let client = ReqwestClient::builder()
            .connect_timeout(connect_timeout)
            .build()
            .map_err(|error| IpGeolocationError::transport("failed to build HTTP client", error))?;

        Ok(Self { client })
    }
}

impl HttpTransport for ReqwestBlockingTransport {
    fn send(
        &self,
        request: &HttpRequestData,
    ) -> Result<(HttpResponseData, u64), IpGeolocationError> {
        let started_at = Instant::now();
        let mut builder = self
            .client
            .request(request.method.clone(), request.url.clone());
        builder = builder.timeout(request.request_timeout);
        builder = builder.headers(to_header_map(&request.headers)?);

        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        let response = builder
            .send()
            .map_err(|error| map_reqwest_error("request timeout exceeded", error))?;

        let status_code = response.status().as_u16();
        let headers = clone_headers(response.headers());
        let body = read_response_body(response, MAX_RESPONSE_BODY_BYTES)?;

        let duration_ms = started_at.elapsed().as_millis() as u64;

        Ok((
            HttpResponseData {
                status_code,
                body,
                headers,
            },
            duration_ms,
        ))
    }
}

fn read_response_body(
    reader: impl Read,
    max_response_body_bytes: usize,
) -> Result<String, IpGeolocationError> {
    let mut buffer = Vec::new();
    reader
        .take((max_response_body_bytes + 1) as u64)
        .read_to_end(&mut buffer)
        .map_err(|error| IpGeolocationError::transport("failed to read response body", error))?;

    if buffer.len() > max_response_body_bytes {
        return Err(IpGeolocationError::transport_message(
            "response body exceeded the maximum allowed size",
        ));
    }

    String::from_utf8(buffer).map_err(|error| {
        IpGeolocationError::serialization("failed to decode response body as UTF-8", error)
    })
}

fn to_header_map(headers: &BTreeMap<String, String>) -> Result<HeaderMap, IpGeolocationError> {
    let mut header_map = HeaderMap::new();
    for (name, value) in headers {
        let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            IpGeolocationError::validation_message(format!("invalid header name {name:?}: {error}"))
        })?;
        let header_value = HeaderValue::from_str(value).map_err(|error| {
            IpGeolocationError::validation_message(format!(
                "invalid header value for {name:?}: {error}"
            ))
        })?;
        header_map.insert(header_name, header_value);
    }

    Ok(header_map)
}

fn clone_headers(headers: &HeaderMap) -> BTreeMap<String, Vec<String>> {
    let mut cloned = BTreeMap::<String, Vec<String>>::new();
    for (name, value) in headers {
        let key = name.as_str().to_string();
        let entry = cloned.entry(key).or_default();
        if let Ok(value) = value.to_str() {
            entry.push(value.to_string());
        }
    }

    cloned
}

fn map_reqwest_error(context: &'static str, error: reqwest::Error) -> IpGeolocationError {
    if error.is_timeout() {
        if error.is_connect() {
            return IpGeolocationError::request_timeout("connect timeout exceeded");
        }

        return IpGeolocationError::request_timeout(context);
    }

    IpGeolocationError::transport(context, error)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::{Cursor, Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    use reqwest::{Method, Url};

    use super::{
        read_response_body, to_header_map, HttpRequestData, HttpTransport, ReqwestBlockingTransport,
    };
    use crate::errors::IpGeolocationError;

    #[test]
    fn invalid_header_name_is_rejected() {
        let mut headers = BTreeMap::new();
        headers.insert("Bad\nHeader".to_string(), "value".to_string());

        let error = to_header_map(&headers).unwrap_err();
        assert!(matches!(error, IpGeolocationError::Validation(_)));
    }

    #[test]
    fn oversized_body_is_rejected() {
        let body = read_response_body(Cursor::new(b"abcdef".to_vec()), 5).unwrap_err();
        assert_eq!(
            body.to_string(),
            "response body exceeded the maximum allowed size"
        );
    }

    #[test]
    fn invalid_utf8_body_is_rejected() {
        let error = read_response_body(Cursor::new(vec![0xff]), 4).unwrap_err();
        assert!(matches!(error, IpGeolocationError::Serialization { .. }));
        assert_eq!(error.to_string(), "failed to decode response body as UTF-8");
    }

    #[test]
    fn request_timeout_is_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_buffer = [0u8; 1024];
            let _ = stream.read(&mut request_buffer);
            thread::sleep(Duration::from_millis(200));
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
            );
        });

        let transport = ReqwestBlockingTransport::new(Duration::from_millis(200)).unwrap();
        let request = HttpRequestData {
            method: Method::GET,
            url: Url::parse(&format!("http://{address}/timeout")).unwrap(),
            headers: BTreeMap::new(),
            body: None,
            request_timeout: Duration::from_millis(50),
        };

        let error = transport.send(&request).unwrap_err();
        assert!(matches!(error, IpGeolocationError::RequestTimeout(_)));
        assert_eq!(error.to_string(), "request timeout exceeded");

        server.join().unwrap();
    }
}
