use std::collections::BTreeMap;

use anybot_protocol::StoredRecord;
use sqlx::sqlite::SqliteConnection;

use crate::error::StoreError;
use crate::model::{ApplyOutcome, LocalRecord, Progress};
use crate::replica_rows::{RawRecord, record_of};
use crate::store::Store;

impl Store {
    pub async fn progress(&self, stream: &str) -> Result<Progress, StoreError> {
        let row = sqlx::query!(
            "SELECT through_seq, through_record_id FROM streams WHERE stream = ?",
            stream
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map_or_else(Progress::default, |found| Progress {
            seq: found.through_seq,
            record_id: found.through_record_id,
        }))
    }

    pub async fn replica_streams(&self) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query!("SELECT stream FROM streams ORDER BY stream")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|row| row.stream).collect())
    }

    pub async fn records(&self, stream: &str) -> Result<Vec<LocalRecord>, StoreError> {
        let raws = sqlx::query_as!(
            RawRecord,
            r#"SELECT stream, seq, record_id, kind, content, created_at_ms, offline_sent,
                task_id, task_seq, purged FROM records WHERE stream = ? ORDER BY seq"#,
            stream
        )
        .fetch_all(&self.pool)
        .await?;
        raws.into_iter().map(record_of).collect()
    }

    pub async fn record_at(
        &self,
        stream: &str,
        seq: i64,
    ) -> Result<Option<LocalRecord>, StoreError> {
        let raw = sqlx::query_as!(
            RawRecord,
            r#"SELECT stream, seq, record_id, kind, content, created_at_ms, offline_sent,
                task_id, task_seq, purged FROM records WHERE stream = ? AND seq = ?"#,
            stream,
            seq
        )
        .fetch_optional(&self.pool)
        .await?;
        raw.map(record_of).transpose()
    }

    pub async fn apply_page(
        &self,
        stream: &str,
        records: &[StoredRecord],
    ) -> Result<ApplyOutcome, StoreError> {
        let mut tx = self.pool.begin().await?;
        ensure_stream(&mut tx, stream).await?;
        let current = through_of(&mut tx, stream).await?;
        let ordered: BTreeMap<i64, &StoredRecord> =
            records.iter().map(|rec| (rec.seq, rec)).collect();
        let mut next = current.saturating_add(1);
        let mut inserted = Vec::new();
        let mut gap = false;
        for (seq, rec) in ordered {
            if seq < next {
                continue;
            }
            if seq > next {
                gap = true;
                break;
            }
            insert_record(&mut tx, stream, rec).await?;
            inserted.push(rec);
            next = next.saturating_add(1);
        }
        if let Some(last) = inserted.last() {
            set_progress(&mut tx, stream, last.seq, &last.device_id).await?;
        }
        for rec in &inserted {
            crate::purge::apply_purge(&mut tx, stream, rec).await?;
        }
        tx.commit().await?;
        Ok(ApplyOutcome {
            inserted: u64::try_from(inserted.len())
                .map_err(|e| StoreError::BadValue(e.to_string()))?,
            through_seq: next.saturating_sub(1),
            gap,
        })
    }

    pub async fn replace_stream(
        &self,
        stream: &str,
        records: &[StoredRecord],
    ) -> Result<(), StoreError> {
        let mut expected = 1i64;
        for rec in records {
            if rec.seq != expected {
                return Err(StoreError::NotContiguous(format!(
                    "{stream}: wanted {expected}, got {}",
                    rec.seq
                )));
            }
            expected = expected.saturating_add(1);
        }
        let mut tx = self.pool.begin().await?;
        ensure_stream(&mut tx, stream).await?;
        sqlx::query!("DELETE FROM records WHERE stream = ?", stream)
            .execute(&mut *tx)
            .await?;
        for rec in records {
            insert_record(&mut tx, stream, rec).await?;
        }
        for rec in records {
            crate::purge::apply_purge(&mut tx, stream, rec).await?;
        }
        let last = records.last();
        let (seq, id) = last.map_or((0, ""), |rec| (rec.seq, rec.device_id.as_str()));
        set_progress(&mut tx, stream, seq, id).await?;
        tx.commit().await?;
        Ok(())
    }
}

async fn ensure_stream(conn: &mut SqliteConnection, stream: &str) -> Result<(), StoreError> {
    sqlx::query!("INSERT OR IGNORE INTO streams (stream) VALUES (?)", stream)
        .execute(conn)
        .await?;
    Ok(())
}

async fn through_of(conn: &mut SqliteConnection, stream: &str) -> Result<i64, StoreError> {
    let row = sqlx::query!("SELECT through_seq FROM streams WHERE stream = ?", stream)
        .fetch_one(conn)
        .await?;
    Ok(row.through_seq)
}

async fn set_progress(
    conn: &mut SqliteConnection,
    stream: &str,
    seq: i64,
    record_id: &str,
) -> Result<(), StoreError> {
    sqlx::query!(
        "UPDATE streams SET through_seq = ?, through_record_id = ? WHERE stream = ?",
        seq,
        record_id,
        stream
    )
    .execute(conn)
    .await?;
    Ok(())
}

async fn insert_record(
    conn: &mut SqliteConnection,
    stream: &str,
    rec: &StoredRecord,
) -> Result<(), StoreError> {
    let content =
        serde_json::to_string(&rec.content).map_err(|e| StoreError::BadValue(e.to_string()))?;
    let offline = i64::from(rec.offline_sent);
    sqlx::query!(
        "INSERT OR REPLACE INTO records (stream, seq, record_id, kind, content, created_at_ms,
            offline_sent, task_id, task_seq, purged) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        stream,
        rec.seq,
        rec.device_id,
        rec.kind,
        content,
        rec.created_at_ms,
        offline,
        rec.task_id,
        rec.task_seq
    )
    .execute(conn)
    .await?;
    Ok(())
}
