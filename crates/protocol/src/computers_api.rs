use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputerView {
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub device_id: String,
    #[cfg_attr(feature = "schema", schema(example = "desk"))]
    pub name: String,
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_000_i64))]
    pub offline_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "schema", schema(example = "computers-offline-no-signal"))]
    pub offline_reason: Option<String>,
    #[cfg_attr(feature = "schema", schema(example = false))]
    pub delegate_enabled: bool,
    #[cfg_attr(feature = "schema", schema(example = false))]
    pub paired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputersView {
    pub computers: Vec<ComputerView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputerRenameRequest {
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub device_id: String,
    #[cfg_attr(feature = "schema", schema(example = "study Mac"))]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputerRemoveRequest {
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ComputerDelegateRequest {
    #[cfg_attr(feature = "schema", schema(example = "018f-device-uuidv7"))]
    pub device_id: String,
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PairingCodeRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PairingCodeReply {
    #[cfg_attr(feature = "schema", schema(example = "048213"))]
    pub code: String,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_600_000_i64))]
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PairRedeemRequest {
    #[cfg_attr(feature = "schema", schema(example = "048213"))]
    pub code: String,
    #[cfg_attr(feature = "schema", schema(example = "server room"))]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PasswordReenterRequest {
    #[cfg_attr(feature = "schema", schema(example = "longenough"))]
    pub password: String,
}
