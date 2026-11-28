use serde::{Deserialize, Serialize};

pub const SSE_EVENT_PROGRESS: &str = "progress";
pub const SSE_EVENT_NEW_RECORDS: &str = "new_records";
pub const SSE_EVENT_TYPING: &str = "typing";
pub const HEARTBEAT_INTERVAL_SECS: u64 = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct StreamProgress {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: String,
    #[cfg_attr(feature = "schema", schema(example = 45_i64))]
    pub seq: i64,
    #[cfg_attr(feature = "schema", schema(example = "018f-uuidv7"))]
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ProgressEvent {
    pub streams: Vec<StreamProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct NewRecordsEvent {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: String,
    #[cfg_attr(feature = "schema", schema(example = 46_i64))]
    pub seq: i64,
    #[cfg_attr(feature = "schema", schema(example = "018f-uuidv7"))]
    pub device_id: String,
}

#[must_use]
pub fn sse_data_line(data: &str) -> String {
    format!("data: {data}\n\n")
}

#[must_use]
pub fn sse_event_line(event: &str, data: &str) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

#[must_use]
pub fn sse_heartbeat_line() -> &'static str {
    ":hb\n\n"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct SseStreamProgress {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: String,
    #[cfg_attr(feature = "schema", schema(example = 45_i64))]
    pub max_seq: i64,
}
