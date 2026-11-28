use anybot_foundation::CallId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ResumeCardStep {
    #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
    pub call_id: CallId,
    pub tool: String,
    pub args_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ResumeCard {
    pub maybe_done: Vec<ResumeCardStep>,
    pub queued_messages: Vec<String>,
}
