use anybot_domain::{DedupFields, dedup_same};
use anybot_messages::MessageCode;
use anybot_protocol::{AppendResult, RecordItem};
use anybot_store_server::streams;

use crate::req_write::{CallerDenied, internal_text};

pub(crate) fn duplicate_result(
    existing: streams::RecordRow,
    item: &RecordItem,
) -> Result<AppendResult, CallerDenied> {
    let stored =
        serde_json::from_str(&existing.content).map_err(|e| internal_text(e.to_string()))?;
    let first = DedupFields {
        kind: existing.kind,
        content: stored,
        task_id: existing.task_id,
        created_at_ms: existing.created_at_ms,
    };
    let second = item_fields(item);
    if existing.purged || dedup_same(&first, &second) {
        return Ok(AppendResult::duplicate(existing.seq));
    }
    Ok(AppendResult::rejected(
        MessageCode::RecordIdConflict.as_str().to_owned(),
    ))
}

fn item_fields(item: &RecordItem) -> DedupFields {
    DedupFields {
        kind: item.kind.clone(),
        content: item.content.clone(),
        task_id: item.task_id.clone(),
        created_at_ms: item.created_at_ms,
    }
}
