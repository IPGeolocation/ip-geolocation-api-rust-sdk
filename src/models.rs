use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IpGeolocationResponse {
    pub ip: Option<String>,
    pub domain: Option<String>,
    pub hostname: Option<String>,
    pub location: Option<Location>,
    pub country_metadata: Option<CountryMetadata>,
    pub network: Option<Network>,
    pub currency: Option<Currency>,
    pub asn: Option<Asn>,
    pub company: Option<Company>,
    pub security: Option<Security>,
    pub abuse: Option<Abuse>,
    pub time_zone: Option<TimeZoneInfo>,
    pub user_agent: Option<UserAgent>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Location {
    pub country_name: Option<String>,
    pub city: Option<String>,
    pub state_prov: Option<String>,
    pub latitude: Option<String>,
    pub longitude: Option<String>,
    pub country_code2: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CountryMetadata {
    pub calling_code: Option<String>,
    pub tld: Option<String>,
    pub languages: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Currency {
    pub code: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Network {
    pub connection_type: Option<String>,
    pub route: Option<String>,
    pub is_anycast: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Asn {
    pub as_number: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub domain: Option<String>,
    pub date_allocated: Option<String>,
    pub rir: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Company {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub domain: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeZoneInfo {
    pub name: Option<String>,
    pub offset: Option<f64>,
    pub offset_with_dst: Option<f64>,
    pub current_time: Option<String>,
    pub current_time_unix: Option<f64>,
    #[serde(alias = "current_timezone_abbreviation")]
    pub current_tz_abbreviation: Option<String>,
    #[serde(alias = "current_timezone_name")]
    pub current_tz_full_name: Option<String>,
    #[serde(alias = "timezone_abbreviation")]
    pub standard_tz_abbreviation: Option<String>,
    #[serde(alias = "timezone_name")]
    pub standard_tz_full_name: Option<String>,
    pub is_dst: Option<bool>,
    pub dst_savings: Option<f64>,
    pub dst_exists: Option<bool>,
    #[serde(alias = "dst_timezone_abbreviation")]
    pub dst_tz_abbreviation: Option<String>,
    #[serde(alias = "dst_timezone_name")]
    pub dst_tz_full_name: Option<String>,
    pub dst_start: Option<DstTransition>,
    pub dst_end: Option<DstTransition>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DstTransition {
    pub utc_time: Option<String>,
    pub duration: Option<String>,
    pub gap: Option<bool>,
    pub date_time_after: Option<String>,
    pub date_time_before: Option<String>,
    pub overlap: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Security {
    pub threat_score: Option<f64>,
    pub is_tor: Option<bool>,
    pub is_proxy: Option<bool>,
    pub proxy_provider_names: Option<Vec<String>>,
    pub proxy_confidence_score: Option<f64>,
    pub proxy_last_seen: Option<String>,
    pub is_residential_proxy: Option<bool>,
    pub is_vpn: Option<bool>,
    pub vpn_provider_names: Option<Vec<String>>,
    pub vpn_confidence_score: Option<f64>,
    pub vpn_last_seen: Option<String>,
    pub is_relay: Option<bool>,
    pub relay_provider_name: Option<String>,
    pub is_anonymous: Option<bool>,
    pub is_known_attacker: Option<bool>,
    pub is_bot: Option<bool>,
    pub is_spam: Option<bool>,
    pub is_cloud_provider: Option<bool>,
    pub cloud_provider_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Abuse {
    pub route: Option<String>,
    pub country: Option<String>,
    pub name: Option<String>,
    pub organization: Option<String>,
    pub kind: Option<String>,
    pub address: Option<String>,
    pub emails: Option<Vec<String>>,
    pub phone_numbers: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserAgent {
    pub user_agent_string: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub version: Option<String>,
    pub version_major: Option<String>,
    pub device: Option<UserAgentDevice>,
    pub engine: Option<UserAgentEngine>,
    pub operating_system: Option<UserAgentOperatingSystem>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserAgentDevice {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub brand: Option<String>,
    pub cpu: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserAgentEngine {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub version: Option<String>,
    pub version_major: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserAgentOperatingSystem {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub version: Option<String>,
    pub version_major: Option<String>,
    pub build: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BulkLookupError {
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BulkLookupResult {
    pub data: Option<IpGeolocationResponse>,
    pub error: Option<BulkLookupError>,
}

impl BulkLookupResult {
    pub fn is_success(&self) -> bool {
        self.data.is_some() && self.error.is_none()
    }
}
