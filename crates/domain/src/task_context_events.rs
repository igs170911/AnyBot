use serde::{Deserialize, Serialize};

use anybot_foundation::FoundationError;
use anybot_protocol::{RawTaskEventKind, TaskEventKind};

pub const CONTEXT_COLLAPSED_KIND: &str = "context_collapsed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCollapseReason {
    Accumulated,
    Budget,
    Compact,
    ModelSwapped,
    ReplaceAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCollapsed {
    pub ids: Vec<String>,
    pub reason: ContextCollapseReason,
}

#[must_use = "record this kind as a task event"]
pub fn collapsed_kind(collapsed: &ContextCollapsed) -> TaskEventKind {
    TaskEventKind::Raw(RawTaskEventKind {
        kind: CONTEXT_COLLAPSED_KIND.to_owned(),
        data: serde_json::to_value(collapsed).unwrap_or(serde_json::Value::Null),
    })
}

pub fn parse_collapsed(kind: &TaskEventKind) -> Result<Option<ContextCollapsed>, FoundationError> {
    let TaskEventKind::Raw(raw) = kind else {
        return Ok(None);
    };
    if raw.kind != CONTEXT_COLLAPSED_KIND {
        return Ok(None);
    }
    serde_json::from_value(raw.data.clone())
        .map(Some)
        .map_err(|e| FoundationError::IdParse(e.to_string()))
}

pub fn parse_memory_dropped(kind: &TaskEventKind) -> Result<Option<Vec<String>>, FoundationError> {
    let TaskEventKind::Raw(raw) = kind else {
        return Ok(None);
    };
    if raw.kind != CONTEXT_MEMORY_DROPPED_KIND {
        return Ok(None);
    }
    serde_json::from_value(raw.data.clone())
        .map(Some)
        .map_err(|e| FoundationError::IdParse(e.to_string()))
}

pub const CONTEXT_MEMORY_DROPPED_KIND: &str = "context_memory_dropped";

#[must_use = "record this kind as a task event"]
pub fn memory_dropped_kind(ids: &[String]) -> TaskEventKind {
    TaskEventKind::Raw(RawTaskEventKind {
        kind: CONTEXT_MEMORY_DROPPED_KIND.to_owned(),
        data: serde_json::to_value(ids).unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
#[path = "task_context_events_tests.rs"]
mod tests;
