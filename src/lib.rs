//! Rust client types for the IPGeolocation.io IP Location API.
//!
//! Homepage: <https://ipgeolocation.io>
//!
//! IP Location API: <https://ipgeolocation.io/ip-location-api.html>
//!
//! Documentation: <https://ipgeolocation.io/documentation/ip-location-api.html>

mod api_response;
mod client;
mod config;
mod errors;
mod models;
mod requests;
mod response_format;
mod transport;
mod version;

pub use api_response::{ApiResponse, ApiResponseMetadata, HeaderValues};
pub use client::IpGeolocationClient;
pub use config::IpGeolocationClientConfig;
pub use errors::{ApiError, IpGeolocationError, ValidationError};
pub use models::{
    Abuse, Asn, BulkLookupError, BulkLookupResult, Company, CountryMetadata, Currency,
    DstTransition, IpGeolocationResponse, Location, Network, Security, TimeZoneInfo, UserAgent,
    UserAgentDevice, UserAgentEngine, UserAgentOperatingSystem,
};
pub use requests::{BulkLookupIpGeolocationRequest, LookupIpGeolocationRequest};
pub use response_format::ResponseFormat;
pub use version::VERSION;
