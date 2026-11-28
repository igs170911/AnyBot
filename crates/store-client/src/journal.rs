use anybot_protocol::ToolEffect;
use serde_json::Value;
use sqlx::sqlite::SqliteConnection;

use crate::error::StoreError;
use crate::model::OutboxState;
use crate::store::Store;

pub const OUTBOX_SEQ_BASE: i64 = 1_000_000_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    pub stream: String,
    pub record_id: String,
    pub kind: String,
    pub content: Value,
    pub local_detail: Option<String>,
    pub created_at_ms: i64,
    pub task_id: Option<String>,
    pub turn: Option<i64>,
    pub effect: Option<ToolEffect>,
    pub offline_at_create: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalRow {
    pub local_id: i64,
    pub stream: String,
    pub record_id: String,
    pub kind: String,
    pub content: Value,
    pub local_detail: Option<String>,
    pub created_at_ms: i64,
    pub task_id: Option<String>,
    pub turn: Option<i64>,
    pub effect: Option<ToolEffect>,
    pub offline_at_create: bool,
    pub state: OutboxState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JournalAppend {
    Stored(Box<JournalRow>),
    Stopped,
    Cancelled,
}

struct RawJournal {
    local_id: i64,
    stream: String,
    record_id: String,
    kind: String,
    content: String,
    local_detail: Option<String>,
    created_at_ms: i64,
    task_id: Option<String>,
    turn: Option<i64>,
    effect: Option<String>,
    offline_at_create: i64,
    state: String,
}

impl Store {
    pub async fn journal_append(&self, entry: &JournalEntry) -> Result<JournalRow, StoreError> {
        let mut conn = self.pool.acquire().await?;
        insert_journal_row(&mut conn, entry).await
    }

    pub async fn journal_append_unless_halted(
        &self,
        entry: &JournalEntry,
        turn_started_seq: i64,
        task_id: &str,
    ) -> Result<JournalAppend, StoreError> {
        let mut tx = self.pool.begin().await?;
        if let Some(halt) =
            halt_in_journal(&mut tx, &entry.stream, task_id, turn_started_seq).await?
        {
            tx.rollback().await?;
            return Ok(halt);
        }
        let row = insert_journal_row(&mut tx, entry).await?;
        tx.commit().await?;
        Ok(JournalAppend::Stored(Box::new(row)))
    }

    pub async fn journal_open(&self) -> Result<Vec<JournalRow>, StoreError> {
        let raws = sqlx::query_as!(
            RawJournal,
            r#"SELECT o.local_id AS "local_id!", o.stream, o.record_id, o.kind, o.content,
                o.local_detail, o.created_at_ms, o.task_id, o.turn, o.effect,
                o.offline_at_create, o.state
                FROM outbox o
                WHERE o.task_id IS NOT NULL
                  AND o.turn IS NOT NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM outbox e
                      WHERE e.task_id = o.task_id
                        AND e.turn = o.turn
                        AND e.kind = 'turn_ended'
                  )
                ORDER BY o.local_id"#
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(journal_row_of).collect()
    }

    pub async fn journal_trim(&self, task_id: &str, turn: i64) -> Result<(), StoreError> {
        sqlx::query!(
            "UPDATE outbox SET local_detail = NULL WHERE task_id = ? AND turn = ?",
            task_id,
            turn
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn journal_stream_rows(&self, stream: &str) -> Result<Vec<JournalRow>, StoreError> {
        let raws = sqlx::query_as!(
            RawJournal,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, local_detail,
                created_at_ms, task_id, turn, effect, offline_at_create, state
                FROM outbox WHERE stream = ? ORDER BY local_id"#,
            stream
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(journal_row_of).collect()
    }

    pub async fn journal_task_rows(&self, task_id: &str) -> Result<Vec<JournalRow>, StoreError> {
        let raws = sqlx::query_as!(
            RawJournal,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, local_detail,
                created_at_ms, task_id, turn, effect, offline_at_create, state
                FROM outbox WHERE task_id = ? ORDER BY local_id"#,
            task_id
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(journal_row_of).collect()
    }
}

async fn insert_journal_row(
    conn: &mut SqliteConnection,
    entry: &JournalEntry,
) -> Result<JournalRow, StoreError> {
    let content =
        serde_json::to_string(&entry.content).map_err(|e| StoreError::BadValue(e.to_string()))?;
    let offline = i64::from(entry.offline_at_create);
    let effect_str = entry.effect.map(effect_to_text);
    let done = sqlx::query!(
        "INSERT INTO outbox (stream, record_id, kind, content, created_at_ms, task_id,
            offline_at_create, state, local_detail, turn, effect)
            VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?)",
        entry.stream,
        entry.record_id,
        entry.kind,
        content,
        entry.created_at_ms,
        entry.task_id,
        offline,
        entry.local_detail,
        entry.turn,
        effect_str
    )
    .execute(conn)
    .await?;
    Ok(JournalRow {
        local_id: done.last_insert_rowid(),
        stream: entry.stream.clone(),
        record_id: entry.record_id.clone(),
        kind: entry.kind.clone(),
        content: entry.content.clone(),
        local_detail: entry.local_detail.clone(),
        created_at_ms: entry.created_at_ms,
        task_id: entry.task_id.clone(),
        turn: entry.turn,
        effect: entry.effect,
        offline_at_create: entry.offline_at_create,
        state: OutboxState::Pending,
    })
}

async fn halt_in_journal(
    conn: &mut SqliteConnection,
    stream: &str,
    task_id: &str,
    turn_started_seq: i64,
) -> Result<Option<JournalAppend>, StoreError> {
    let min_local = if turn_started_seq >= OUTBOX_SEQ_BASE {
        turn_started_seq.saturating_sub(OUTBOX_SEQ_BASE)
    } else {
        let server_rows = sqlx::query!(
            "SELECT kind FROM records WHERE stream = ? AND task_id = ? AND seq > ?
                AND kind IN ('task_stopped', 'task_cancelled') ORDER BY seq",
            stream,
            task_id,
            turn_started_seq
        )
        .fetch_all(&mut *conn)
        .await?;
        if let Some(found) = pick_halt(server_rows.into_iter().map(|r| r.kind)) {
            return Ok(Some(found));
        }
        0
    };
    let local_rows = sqlx::query!(
        "SELECT kind FROM outbox WHERE stream = ? AND task_id = ? AND local_id > ?
            AND kind IN ('task_stopped', 'task_cancelled') ORDER BY local_id",
        stream,
        task_id,
        min_local
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(pick_halt(local_rows.into_iter().map(|r| r.kind)))
}

fn pick_halt<I>(kinds: I) -> Option<JournalAppend>
where
    I: IntoIterator<Item = String>,
{
    let mut stopped = false;
    for kind in kinds {
        if kind == "task_cancelled" {
            return Some(JournalAppend::Cancelled);
        }
        if kind == "task_stopped" {
            stopped = true;
        }
    }
    if stopped {
        Some(JournalAppend::Stopped)
    } else {
        None
    }
}

fn journal_row_of(raw: RawJournal) -> Result<JournalRow, StoreError> {
    let content = serde_json::from_str(&raw.content)
        .map_err(|e| StoreError::BadValue(format!("journal content: {e}")))?;
    let state = match raw.state.as_str() {
        "pending" => OutboxState::Pending,
        "rejected" => OutboxState::Rejected,
        other => return Err(StoreError::BadValue(format!("journal state {other}"))),
    };
    let effect = raw.effect.as_deref().map(effect_from_text).transpose()?;
    Ok(JournalRow {
        local_id: raw.local_id,
        stream: raw.stream,
        record_id: raw.record_id,
        kind: raw.kind,
        content,
        local_detail: raw.local_detail,
        created_at_ms: raw.created_at_ms,
        task_id: raw.task_id,
        turn: raw.turn,
        effect,
        offline_at_create: raw.offline_at_create != 0,
        state,
    })
}

fn effect_to_text(effect: ToolEffect) -> &'static str {
    match effect {
        ToolEffect::ReadOnly => "read_only",
        ToolEffect::Repeatable => "repeatable",
        ToolEffect::Irreversible => "irreversible",
    }
}

fn effect_from_text(text: &str) -> Result<ToolEffect, StoreError> {
    match text {
        "read_only" => Ok(ToolEffect::ReadOnly),
        "repeatable" => Ok(ToolEffect::Repeatable),
        "irreversible" => Ok(ToolEffect::Irreversible),
        other => Err(StoreError::BadValue(format!("journal effect {other}"))),
    }
}
