use serde::{Deserialize, Serialize};

pub const MAX_APPEND_ITEMS: usize = 1_000;
pub const MAX_AFTER_PAGE: i64 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum StreamSelector {
    Account,
    Bot {
        #[cfg_attr(feature = "schema", schema(example = "018f-bot-uuidv7"))]
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RecordItem {
    #[cfg_attr(feature = "schema", schema(example = "018f-uuidv7"))]
    pub device_id: String,
    #[cfg_attr(feature = "schema", schema(example = "message"))]
    pub kind: String,
    pub content: serde_json::Value,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_100_i64))]
    pub created_at_ms: i64,
    #[cfg_attr(feature = "schema", schema(example = false))]
    pub offline_sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "018f-task-uuidv7"))]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct AppendRequest {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    pub items: Vec<RecordItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub struct AppendResult {
    #[cfg_attr(feature = "schema", schema(example = "accepted"))]
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = 1_i64))]
    pub seq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "record-id-conflict"))]
    pub code: Option<String>,
}

impl AppendResult {
    #[must_use]
    pub fn accepted(seq: i64) -> Self {
        Self {
            result: "accepted".to_owned(),
            seq: Some(seq),
            code: None,
        }
    }

    #[must_use]
    pub fn duplicate(seq: i64) -> Self {
        Self {
            result: "duplicate".to_owned(),
            seq: Some(seq),
            code: None,
        }
    }

    #[must_use]
    pub fn rejected(code: String) -> Self {
        Self {
            result: "rejected".to_owned(),
            seq: None,
            code: Some(code),
        }
    }

    #[must_use]
    pub fn accepted_seq(&self) -> Option<i64> {
        if self.result == "accepted" {
            self.seq
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct AppendReply {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    pub results: Vec<AppendResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct AfterRequest {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    #[cfg_attr(feature = "schema", schema(example = 2_i64))]
    pub after_seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "018f-uuidv7"))]
    pub device_id_at_after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct StoredRecord {
    #[cfg_attr(feature = "schema", schema(example = 3_i64))]
    pub seq: i64,
    #[cfg_attr(feature = "schema", schema(example = "018f-uuidv7"))]
    pub device_id: String,
    #[cfg_attr(feature = "schema", schema(example = "message"))]
    pub kind: String,
    pub content: serde_json::Value,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_100_i64))]
    pub created_at_ms: i64,
    #[cfg_attr(feature = "schema", schema(example = false))]
    pub offline_sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "018f-task-uuidv7"))]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = 7_i64))]
    pub task_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct AfterPage {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    pub records: Vec<StoredRecord>,
    #[cfg_attr(feature = "schema", schema(example = false))]
    pub has_more: bool,
    #[cfg_attr(feature = "schema", schema(example = 45_i64))]
    pub max_seq: i64,
}

#[must_use]
pub fn selector_key(selector: &StreamSelector, owner: &str) -> String {
    match selector {
        StreamSelector::Account => format!("account/{owner}"),
        StreamSelector::Bot { id } => format!("bot/{id}"),
    }
}

pub fn selector_from_key(key: &str) -> Result<StreamSelector, String> {
    if let Some(owner) = key.strip_prefix("account/") {
        if owner.is_empty() {
            return Err("account key needs an owner".to_owned());
        }
        return Ok(StreamSelector::Account);
    }
    if let Some(id) = key.strip_prefix("bot/") {
        if id.is_empty() {
            return Err("bot key needs an id".to_owned());
        }
        return Ok(StreamSelector::Bot { id: id.to_owned() });
    }
    Err(format!("unknown stream key: {key}"))
}
