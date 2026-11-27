use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TwoStepChallenge {
    #[cfg_attr(feature = "schema", schema(example = "9f2c4e7a1b3d5f60"))]
    pub challenge: String,
    #[cfg_attr(feature = "schema", schema(example = "alice"))]
    pub account: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TwoStepLoginRequest {
    #[cfg_attr(feature = "schema", schema(example = "9f2c4e7a1b3d5f60"))]
    pub challenge: String,
    #[cfg_attr(feature = "schema", schema(example = "123456"))]
    pub code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TwoStepCodeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schema(example = "123456"))]
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct TwoStepSetup {
    #[cfg_attr(
        feature = "schema",
        schema(
            example = "otpauth://totp/AnyBot:alice?secret=GEZDGNBVGY3TQOJQ&issuer=AnyBot&period=30&digits=6"
        )
    )]
    pub otpauth_uri: String,
    #[cfg_attr(feature = "schema", schema(example = "GEZDGNBVGY3TQOJQ"))]
    pub secret: String,
    #[cfg_attr(feature = "schema", schema(example = json!(["0a1b2c3d4e", "5f6a7b8c9d"])))]
    pub recovery_codes: Vec<String>,
}
