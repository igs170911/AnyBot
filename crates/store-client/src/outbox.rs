use serde_json::Value;

use crate::error::StoreError;
use crate::model::{NewOutbox, OutboxRow, OutboxState};
use crate::outbox_rows::{RawOutbox, row_of};
use crate::store::Store;

fn text_of(content: &Value) -> Result<String, StoreError> {
    serde_json::to_string(content).map_err(|e| StoreError::BadValue(e.to_string()))
}

fn flag(on: bool) -> i64 {
    i64::from(on)
}

impl Store {
    pub async fn enqueue(&self, item: &NewOutbox) -> Result<(), StoreError> {
        let content = text_of(&item.content)?;
        let offline = flag(item.offline_at_create);
        sqlx::query!(
            "INSERT INTO outbox (stream, record_id, kind, content, created_at_ms, task_id,
                offline_at_create, state) VALUES (?, ?, ?, ?, ?, ?, ?, 'pending')",
            item.stream,
            item.record_id,
            item.kind,
            content,
            item.created_at_ms,
            item.task_id,
            offline
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn import_outbox_row(
        &self,
        row: &OutboxRow,
        local_detail: Option<&str>,
        turn: Option<i64>,
        effect: Option<&str>,
    ) -> Result<(), StoreError> {
        let content = text_of(&row.content)?;
        let offline = flag(row.offline_at_create);
        let sent = row.offline_sent.map(flag);
        let state = row.state.as_text();
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "INSERT OR IGNORE INTO outbox (stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            row.stream,
            row.record_id,
            row.kind,
            content,
            row.created_at_ms,
            row.task_id,
            offline,
            row.attempted_at_ms,
            sent,
            state,
            row.reject_code,
            row.rejected_at_ms
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE outbox SET local_detail = ?, turn = ?, effect = ? WHERE stream = ? AND record_id = ?",
            local_detail,
            turn,
            effect,
            row.stream,
            row.record_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn outbox_all(&self) -> Result<Vec<OutboxRow>, StoreError> {
        let raws = sqlx::query_as!(
            RawOutbox,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms FROM outbox ORDER BY local_id"#
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(row_of).collect()
    }

    pub async fn outbox_pending(
        &self,
        stream: &str,
        limit: i64,
    ) -> Result<Vec<OutboxRow>, StoreError> {
        let raws = sqlx::query_as!(
            RawOutbox,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms FROM outbox WHERE stream = ? AND state = 'pending'
                ORDER BY local_id LIMIT ?"#,
            stream,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(row_of).collect()
    }

    pub async fn outbox_streams(&self) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query!(
            "SELECT DISTINCT stream FROM outbox WHERE state = 'pending' ORDER BY stream"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.stream).collect())
    }

    pub async fn outbox_counts(&self) -> Result<(i64, i64), StoreError> {
        let row = sqlx::query!(
            r#"SELECT COALESCE(SUM(state = 'pending'), 0) AS "pending!: i64",
                COALESCE(SUM(state = 'rejected'), 0) AS "rejected!: i64" FROM outbox"#
        )
        .fetch_one(&self.pool)
        .await?;
        Ok((row.pending, row.rejected))
    }

    pub async fn outbox_mark_attempt(
        &self,
        local_id: i64,
        now_ms: i64,
        offline_sent: bool,
        content: &Value,
    ) -> Result<bool, StoreError> {
        let text = text_of(content)?;
        let flagged = flag(offline_sent);
        let done = sqlx::query!(
            "UPDATE outbox SET attempted_at_ms = COALESCE(attempted_at_ms, ?),
                offline_sent = COALESCE(offline_sent, ?), content = ? WHERE local_id = ?",
            now_ms,
            flagged,
            text,
            local_id
        )
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn outbox_clear_attempt(&self, local_id: i64) -> Result<(), StoreError> {
        sqlx::query!(
            "UPDATE outbox SET attempted_at_ms = NULL, offline_sent = NULL WHERE local_id = ?",
            local_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn outbox_accept(&self, local_id: i64) -> Result<(), StoreError> {
        sqlx::query!("DELETE FROM outbox WHERE local_id = ?", local_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn outbox_reject(
        &self,
        local_id: i64,
        code: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let state = OutboxState::Rejected.as_text();
        sqlx::query!(
            "UPDATE outbox SET state = ?, reject_code = ?, rejected_at_ms = ? WHERE local_id = ?",
            state,
            code,
            now_ms,
            local_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn outbox_cancel_unsent(
        &self,
        record_id: &str,
    ) -> Result<Option<OutboxRow>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let found = sqlx::query_as!(
            RawOutbox,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms FROM outbox WHERE record_id = ? AND state = 'pending'
                AND attempted_at_ms IS NULL"#,
            record_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(raw) = found else {
            tx.commit().await?;
            return Ok(None);
        };
        let id = raw.local_id;
        sqlx::query!("DELETE FROM outbox WHERE local_id = ?", id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        row_of(raw).map(Some)
    }

    pub async fn outbox_take_back_rejected(
        &self,
        record_id: &str,
    ) -> Result<Option<OutboxRow>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let found = sqlx::query_as!(
            RawOutbox,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms FROM outbox WHERE record_id = ? AND state = 'rejected'"#,
            record_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some(raw) = found else {
            tx.commit().await?;
            return Ok(None);
        };
        let id = raw.local_id;
        sqlx::query!("DELETE FROM outbox WHERE local_id = ?", id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        row_of(raw).map(Some)
    }

    pub async fn outbox_drop_expired(&self, cutoff_ms: i64) -> Result<u64, StoreError> {
        let done = sqlx::query!(
            "DELETE FROM outbox WHERE state = 'rejected' AND rejected_at_ms <= ?",
            cutoff_ms
        )
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected())
    }
}
