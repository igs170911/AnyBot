use crate::error::StoreError;
use crate::model::LocalRecord;

pub(crate) struct RawRecord {
    pub stream: String,
    pub seq: i64,
    pub record_id: String,
    pub kind: String,
    pub content: String,
    pub created_at_ms: i64,
    pub offline_sent: i64,
    pub task_id: Option<String>,
    pub task_seq: Option<i64>,
    pub purged: i64,
}

pub(crate) fn record_of(raw: RawRecord) -> Result<LocalRecord, StoreError> {
    let content = serde_json::from_str(&raw.content)
        .map_err(|e| StoreError::BadValue(format!("record content: {e}")))?;
    Ok(LocalRecord {
        stream: raw.stream,
        seq: raw.seq,
        record_id: raw.record_id,
        kind: raw.kind,
        content,
        created_at_ms: raw.created_at_ms,
        offline_sent: raw.offline_sent != 0,
        task_id: raw.task_id,
        task_seq: raw.task_seq,
        purged: raw.purged != 0,
    })
}
