use serde::{Deserialize, Serialize};

pub const SETTING_KIND: &str = "setting";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct SettingContent {
    #[cfg_attr(feature = "schema", schema(example = "accent_color"))]
    pub field: String,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schema(example = "018f-chat-uuidv7"))]
    pub item: Option<String>,
    #[cfg_attr(feature = "schema", schema(example = "green"))]
    pub value: String,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_000_i64))]
    pub changed_at_ms: i64,
    #[cfg_attr(feature = "schema", schema(example = -4000_i64))]
    pub offset_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct SettingEntry {
    #[cfg_attr(feature = "schema", schema(example = "accent_color"))]
    pub field: String,
    #[serde(default)]
    #[cfg_attr(feature = "schema", schema(example = "018f-chat-uuidv7"))]
    pub item: Option<String>,
    #[cfg_attr(feature = "schema", schema(example = "green"))]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct SettingsView {
    pub settings: Vec<SettingEntry>,
}
