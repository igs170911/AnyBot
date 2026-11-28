use serde::{Deserialize, Serialize};
use serde_json::Value;

use anybot_foundation::{EventId, TimestampMs};

use crate::{RawTaskEventKind, TaskDetail, TaskEvent, TaskEventKind, TaskStatus};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusReply {
    #[cfg_attr(feature = "schema", schema(example = "completed"))]
    pub status: TaskStatus,
    #[cfg_attr(
        feature = "schema",
        schema(example = json!({"detail": "user_confirmed"}))
    )]
    pub detail: Option<TaskDetail>,
    #[cfg_attr(
        feature = "schema",
        schema(example = json!({"userConfirmed": true, "userConfirmedAtMs": 1_700_000_000_200_i64}))
    )]
    pub report: TaskReport,
    #[cfg_attr(feature = "schema", schema(example = 9_u64))]
    pub last_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct TaskReport {
    #[cfg_attr(feature = "schema", schema(example = true))]
    pub user_confirmed: bool,
    #[cfg_attr(feature = "schema", schema(example = 1_700_000_000_200_i64))]
    pub user_confirmed_at_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct StatusRecord {
    pub task_seq: i64,
    pub device_id: String,
    pub kind: String,
    pub content: String,
    pub created_at_ms: i64,
}

impl StatusRecord {
    pub fn task_event(&self) -> Result<TaskEvent, String> {
        let content: Value =
            serde_json::from_str(&self.content).map_err(|e| format!("record content: {e}"))?;
        let task_entry = content.get("entry").and_then(Value::as_str) == Some("task");
        let kind = kind_from_content(&content, &self.kind, task_entry)?;
        let event_text = event_id_text(&content, &self.device_id, task_entry);
        let event_id = EventId::parse(event_text).map_err(|e| format!("record event id: {e}"))?;
        let seq = u64::try_from(self.task_seq).map_err(|e| e.to_string())?;
        let at = u64::try_from(self.created_at_ms).map_err(|e| e.to_string())?;
        Ok(TaskEvent {
            event_id,
            seq,
            at_ms: TimestampMs::from_millis(at),
            kind,
        })
    }
}

fn event_id_text<'a>(content: &'a Value, device_id: &'a str, task_entry: bool) -> &'a str {
    if !task_entry {
        return device_id;
    }
    content
        .get("event_id")
        .and_then(Value::as_str)
        .unwrap_or(device_id)
}

fn kind_from_content(
    content: &Value,
    record_kind: &str,
    task_entry: bool,
) -> Result<TaskEventKind, String> {
    let source = if task_entry {
        content.get("kind").cloned().unwrap_or(Value::Null)
    } else {
        serde_json::json!({ "kind": record_kind, "data": content })
    };
    if let Ok(event) = serde_json::from_value::<TaskEventKind>(source.clone()) {
        return Ok(event);
    }
    if task_entry {
        return Err("task entry content has no parsable kind".to_owned());
    }
    let data = content
        .as_object()
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .ok_or_else(|| "record content is not an object".to_owned())?;
    Ok(TaskEventKind::Raw(RawTaskEventKind {
        kind: record_kind.to_owned(),
        data,
    }))
}

pub fn task_event_from_records(records: &[StatusRecord]) -> Result<Vec<TaskEvent>, String> {
    let mut sorted: Vec<&StatusRecord> = records.iter().collect();
    sorted.sort_by_key(|row| row.task_seq);
    sorted.iter().map(|row| row.task_event()).collect()
}
