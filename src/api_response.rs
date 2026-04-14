use std::collections::BTreeMap;

pub type HeaderValues = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApiResponseMetadata {
    pub credits_charged: Option<u32>,
    pub successful_records: Option<u32>,
    pub status_code: u16,
    pub duration_ms: u64,
    pub raw_headers: HeaderValues,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiResponse<T> {
    pub data: T,
    pub metadata: ApiResponseMetadata,
}
