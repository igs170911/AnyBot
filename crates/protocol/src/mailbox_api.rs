use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RoundRequest {
    #[cfg_attr(feature = "schema", schema(example = "018f-bot-uuidv7"))]
    pub bot: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub computer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RoundReply {
    #[cfg_attr(feature = "schema", schema(example = "018f-letter-uuidv7"))]
    pub letter_id: String,
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "schema", schema(example = "offline"))]
    pub reassigned: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ClaimRequest {
    #[cfg_attr(feature = "schema", schema(example = "018f-letter-uuidv7"))]
    pub letter_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ClaimReply {
    #[cfg_attr(feature = "schema", schema(example = "018f-letter-uuidv7"))]
    pub letter_id: String,
    #[cfg_attr(feature = "schema", schema(example = "round"))]
    pub kind: String,
    #[cfg_attr(feature = "schema", schema(value_type = Object))]
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct MailHint {
    #[cfg_attr(feature = "schema", schema(example = "018f-letter-uuidv7"))]
    pub letter_id: String,
    #[cfg_attr(feature = "schema", schema(example = "round"))]
    pub kind: String,
}
