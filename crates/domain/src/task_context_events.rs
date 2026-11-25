use serde::{Deserialize, Serialize};

use anybot_protocol::{RawTaskEventKind, TaskEventKind};

pub const CONTEXT_COLLAPSED_KIND: &str = "context_collapsed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCollapseReason {
    Accumulated,
    Budget,
    Compact,
    ModelSwapped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextCollapsed {
    pub ids: Vec<String>,
    pub reason: ContextCollapseReason,
}

#[must_use]
pub fn collapsed_kind(collapsed: &ContextCollapsed) -> TaskEventKind {
    TaskEventKind::Raw(RawTaskEventKind {
        kind: CONTEXT_COLLAPSED_KIND.to_owned(),
        data: serde_json::to_value(collapsed).unwrap_or(serde_json::Value::Null),
    })
}

#[must_use]
pub fn parse_collapsed(kind: &TaskEventKind) -> Option<ContextCollapsed> {
    let TaskEventKind::Raw(raw) = kind else {
        return None;
    };
    if raw.kind != CONTEXT_COLLAPSED_KIND {
        return None;
    }
    serde_json::from_value(raw.data.clone()).ok()
}

#[cfg(test)]
#[path = "task_context_events_tests.rs"]
mod tests;
