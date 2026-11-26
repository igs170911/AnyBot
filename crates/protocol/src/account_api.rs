use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RegisterRequest {
    #[cfg_attr(feature = "schema", schema(example = "bob"))]
    pub account: String,
    #[cfg_attr(feature = "schema", schema(example = "longenough"))]
    pub password: String,
    #[cfg_attr(feature = "schema", schema(example = "2kqj-7mnp"))]
    pub invite: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct RegisterAck {
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub created: bool,
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub admin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct LoginRequest {
    #[cfg_attr(feature = "schema", schema(example = "alice"))]
    pub account: String,
    #[cfg_attr(feature = "schema", schema(example = "longenough"))]
    pub password: String,
    #[cfg_attr(feature = "schema", schema(example = "phone P"))]
    pub label: String,
    #[cfg_attr(feature = "schema", schema(example = "bearer"))]
    pub credential: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct LoginSession {
    #[serde(skip)]
    pub session_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "a1b2c3d4"))]
    pub bearer_key: Option<String>,
    #[cfg_attr(feature = "schema", schema(example = "alice"))]
    pub account: String,
    #[cfg_attr(feature = "schema", schema(example = "d1"))]
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PasswordChangeRequest {
    #[cfg_attr(feature = "schema", schema(example = "longenough"))]
    pub old: String,
    #[cfg_attr(feature = "schema", schema(example = "newlongenough"))]
    pub new: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ActionAck {
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct InviteResponse {
    #[cfg_attr(feature = "schema", schema(example = "2kqj-7mnp"))]
    pub invite: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct PublicUrlRequest {
    #[cfg_attr(feature = "schema", schema(example = "https://bot.example.test"))]
    pub public_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct ResetPasswordRequest {
    #[cfg_attr(feature = "schema", schema(example = "newlongenough"))]
    pub new: String,
}
