use crate::error::StoreError;
use crate::model::{OutboxRow, OutboxState};

pub(crate) struct RawOutbox {
    pub local_id: i64,
    pub stream: String,
    pub record_id: String,
    pub kind: String,
    pub content: String,
    pub created_at_ms: i64,
    pub task_id: Option<String>,
    pub offline_at_create: i64,
    pub attempted_at_ms: Option<i64>,
    pub offline_sent: Option<i64>,
    pub state: String,
    pub reject_code: Option<String>,
    pub rejected_at_ms: Option<i64>,
}

pub(crate) fn row_of(raw: RawOutbox) -> Result<OutboxRow, StoreError> {
    let content = serde_json::from_str(&raw.content)
        .map_err(|e| StoreError::BadValue(format!("outbox content: {e}")))?;
    let state = match raw.state.as_str() {
        "pending" => OutboxState::Pending,
        "rejected" => OutboxState::Rejected,
        other => return Err(StoreError::BadValue(format!("outbox state {other}"))),
    };
    Ok(OutboxRow {
        local_id: raw.local_id,
        stream: raw.stream,
        record_id: raw.record_id,
        kind: raw.kind,
        content,
        created_at_ms: raw.created_at_ms,
        task_id: raw.task_id,
        offline_at_create: raw.offline_at_create != 0,
        attempted_at_ms: raw.attempted_at_ms,
        offline_sent: raw.offline_sent.map(|flag| flag != 0),
        state,
        reject_code: raw.reject_code,
        rejected_at_ms: raw.rejected_at_ms,
    })
}
