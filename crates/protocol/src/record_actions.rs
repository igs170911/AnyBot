use serde::{Deserialize, Serialize};

use crate::streams_api::StreamSelector;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", schema(example = "delete"))]
pub enum RecordAction {
    Delete,
    Clear,
    Restore,
    PurgePermanent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RecordActionRequest {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    pub action: RecordAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = 3_i64))]
    pub target_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RecordActionReply {
    #[cfg_attr(feature = "schema", schema(example = "account"))]
    pub stream: StreamSelector,
    #[cfg_attr(feature = "schema", schema(example = 9_i64))]
    pub seq: i64,
}
