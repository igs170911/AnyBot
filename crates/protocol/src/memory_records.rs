use anybot_foundation::{BotId, EvidenceId, RecordId, TaskId};
use serde::{Deserialize, Serialize};

pub const MEMORY_ADDED_KIND: &str = "memory_added";
pub const MEMORY_SUPERSEDED_KIND: &str = "memory_superseded";
pub const MEMORY_RETRACTED_KIND: &str = "memory_retracted";
pub const MEMORY_CONFIRMED_KIND: &str = "memory_confirmed";
pub const MEMORY_PINNED_KIND: &str = "memory_pinned";
pub const MEMORY_UNPINNED_KIND: &str = "memory_unpinned";
pub const MEMORY_VOIDED_KIND: &str = "memory_voided";
pub const MEMORY_RESTORED_KIND: &str = "memory_restored";

pub const MEMORY_RECORD_KINDS: [&str; 8] = [
    MEMORY_ADDED_KIND,
    MEMORY_SUPERSEDED_KIND,
    MEMORY_RETRACTED_KIND,
    MEMORY_CONFIRMED_KIND,
    MEMORY_PINNED_KIND,
    MEMORY_UNPINNED_KIND,
    MEMORY_VOIDED_KIND,
    MEMORY_RESTORED_KIND,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemoryEventKind {
    Added,
    Superseded,
    Retracted,
    Confirmed,
    Pinned,
    Unpinned,
    Voided,
    Restored,
}

impl MemoryEventKind {
    #[must_use]
    pub fn kind_name(self) -> &'static str {
        match self {
            MemoryEventKind::Added => MEMORY_ADDED_KIND,
            MemoryEventKind::Superseded => MEMORY_SUPERSEDED_KIND,
            MemoryEventKind::Retracted => MEMORY_RETRACTED_KIND,
            MemoryEventKind::Confirmed => MEMORY_CONFIRMED_KIND,
            MemoryEventKind::Pinned => MEMORY_PINNED_KIND,
            MemoryEventKind::Unpinned => MEMORY_UNPINNED_KIND,
            MemoryEventKind::Voided => MEMORY_VOIDED_KIND,
            MemoryEventKind::Restored => MEMORY_RESTORED_KIND,
        }
    }

    #[must_use]
    pub fn from_kind_name(kind: &str) -> Option<Self> {
        match kind {
            MEMORY_ADDED_KIND => Some(MemoryEventKind::Added),
            MEMORY_SUPERSEDED_KIND => Some(MemoryEventKind::Superseded),
            MEMORY_RETRACTED_KIND => Some(MemoryEventKind::Retracted),
            MEMORY_CONFIRMED_KIND => Some(MemoryEventKind::Confirmed),
            MEMORY_PINNED_KIND => Some(MemoryEventKind::Pinned),
            MEMORY_UNPINNED_KIND => Some(MemoryEventKind::Unpinned),
            MEMORY_VOIDED_KIND => Some(MemoryEventKind::Voided),
            MEMORY_RESTORED_KIND => Some(MemoryEventKind::Restored),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Preference,
    Knowledge,
    Event,
    TaskExperience,
    Lesson,
    Procedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemoryOrigin {
    Remembered,
    TaskAuto,
    Consolidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    UserSaid,
    ToolEvidence,
    ModelInference,
    OtherBot,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    Bot(BotId),
    AboutYou,
}

#[cfg(feature = "schema")]
impl utoipa::PartialSchema for MemoryScope {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        let bot_obj = utoipa::openapi::ObjectBuilder::new()
            .property(
                "bot",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::String)
                    .format(Some(utoipa::openapi::SchemaFormat::KnownFormat(
                        utoipa::openapi::KnownFormat::Uuid,
                    ))),
            )
            .required("bot");
        let about = utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .enum_values(Some(["about_you"]));
        utoipa::openapi::schema::OneOfBuilder::new()
            .item(bot_obj)
            .item(about)
            .into()
    }
}

#[cfg(feature = "schema")]
impl utoipa::ToSchema for MemoryScope {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MemoryEvidenceRef {
    UserRecord {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        record_id: RecordId,
    },
    TaskEvidence {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        task_id: TaskId,
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        evidence_id: EvidenceId,
    },
    Task {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        task_id: TaskId,
    },
    Memory {
        memory_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
pub struct MemoryEventContent {
    pub memory_id: String,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MemoryKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<MemoryOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<MemorySource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<MemoryEvidenceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<MemoryScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contradicts: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[cfg(test)]
#[path = "memory_records_tests.rs"]
mod tests;
