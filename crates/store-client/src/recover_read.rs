use std::path::{Path, PathBuf};

use sqlx::Connection as _;
use sqlx::sqlite::SqliteConnection;

use crate::error::StoreError;
use crate::model::OutboxRow;
use crate::open::{MIGRATOR, connect_pool, immutable_options};
use crate::outbox_rows::{RawOutbox, row_of};
use crate::recover::RecoveryReason;

const NOT_A_DATABASE: &str = "26";

pub(crate) struct JournalSaved {
    pub local_id: i64,
    pub local_detail: Option<String>,
    pub turn: Option<i64>,
    pub effect: Option<String>,
}

pub(crate) struct Rescued {
    pub rows: Vec<OutboxRow>,
    pub journal: Vec<JournalSaved>,
    pub unrecovered: Vec<i64>,
    pub unreadable: bool,
}

impl Rescued {
    pub(crate) fn journal_for(&self, local_id: i64) -> Option<&JournalSaved> {
        self.journal.iter().find(|row| row.local_id == local_id)
    }
}

pub(crate) async fn probe(path: &Path) -> Result<Option<RecoveryReason>, StoreError> {
    let options = immutable_options(path)?;
    let mut conn = match SqliteConnection::connect_with(&options).await {
        Ok(conn) => conn,
        Err(err) => return Ok(Some(classify(&err))),
    };
    let verdict = check_connection(&mut conn).await;
    let _closed = conn.close().await;
    verdict
}

async fn check_connection(
    conn: &mut SqliteConnection,
) -> Result<Option<RecoveryReason>, StoreError> {
    let checked = sqlx::query_scalar::<_, String>("PRAGMA quick_check")
        .fetch_all(&mut *conn)
        .await;
    match checked {
        Err(err) => return Ok(Some(classify(&err))),
        Ok(rows) if rows.iter().any(|row| row != "ok") => {
            return Ok(Some(RecoveryReason::Damaged(rows.join("; "))));
        }
        Ok(_) => {}
    }
    unknown_migration(conn).await
}

async fn unknown_migration(
    conn: &mut SqliteConnection,
) -> Result<Option<RecoveryReason>, StoreError> {
    let listed =
        sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut *conn)
            .await;
    let versions = match listed {
        Ok(versions) => versions,
        Err(err) if is_missing_table(&err) => return Ok(None),
        Err(err) => return Ok(Some(classify(&err))),
    };
    for version in versions {
        if !MIGRATOR.iter().any(|m| m.version == version) {
            let text = format!("migration {version} is newer than this version knows");
            return Ok(Some(RecoveryReason::NewerSchema(text)));
        }
    }
    Ok(None)
}

fn is_missing_table(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .is_some_and(|db| db.message().contains("no such table"))
}

fn classify(err: &sqlx::Error) -> RecoveryReason {
    let code = err
        .as_database_error()
        .and_then(|db| db.code().map(std::borrow::Cow::into_owned));
    if code.as_deref() == Some(NOT_A_DATABASE) {
        RecoveryReason::NotSqlite(err.to_string())
    } else {
        RecoveryReason::Damaged(err.to_string())
    }
}

pub(crate) async fn rescue_outbox(data_dir: &Path, path: &Path, stamp_ms: i64) -> Rescued {
    let nothing = Rescued {
        rows: Vec::new(),
        journal: Vec::new(),
        unrecovered: Vec::new(),
        unreadable: true,
    };
    let Ok(scratch) = copy_to_scratch(data_dir, path, stamp_ms) else {
        return nothing;
    };
    let result = read_scratch(&scratch.join(file_name(path))).await;
    let _removed = std::fs::remove_dir_all(&scratch);
    result.unwrap_or(nothing)
}

fn file_name(path: &Path) -> PathBuf {
    path.file_name().map(PathBuf::from).unwrap_or_default()
}

fn copy_to_scratch(data_dir: &Path, path: &Path, stamp_ms: i64) -> Result<PathBuf, StoreError> {
    let mut counter = 0u32;
    let scratch = loop {
        let candidate = data_dir.join(format!("rescue-scratch-{stamp_ms}-{counter}"));
        if !candidate.exists() {
            break candidate;
        }
        counter = counter.saturating_add(1);
    };
    std::fs::create_dir_all(&scratch)?;
    let base = path.to_string_lossy().into_owned();
    for tail in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{base}{tail}"));
        if from.exists() {
            let name = format!("{}{tail}", file_name(path).to_string_lossy());
            std::fs::copy(&from, scratch.join(name))?;
        }
    }
    Ok(scratch)
}

async fn read_scratch(copy: &Path) -> Option<Rescued> {
    let pool = connect_pool(copy).await.ok()?;
    let journal = load_journal_saved(&pool).await;
    let all = sqlx::query_as!(
        RawOutbox,
        r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
            task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
            rejected_at_ms FROM outbox ORDER BY local_id"#
    )
    .fetch_all(&pool)
    .await;
    let rescued = match all {
        Ok(raws) => Some(collect_rows(raws)),
        Err(_) => read_by_id(&pool).await,
    };
    let rescued = rescued.map(|mut saved| {
        saved.journal = journal;
        saved
    });
    pool.close().await;
    rescued
}

fn collect_rows(found: Vec<RawOutbox>) -> Rescued {
    let mut rows = Vec::new();
    let mut unrecovered = Vec::new();
    for raw in found {
        let id = raw.local_id;
        match row_of(raw) {
            Ok(row) => rows.push(row),
            Err(_) => unrecovered.push(id),
        }
    }
    Rescued {
        rows,
        journal: Vec::new(),
        unrecovered,
        unreadable: false,
    }
}

async fn read_by_id(pool: &sqlx::SqlitePool) -> Option<Rescued> {
    let top = sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(local_id), 0) FROM outbox")
        .fetch_one(pool)
        .await
        .ok()?;
    let mut rows = Vec::new();
    let mut unrecovered = Vec::new();
    for id in 1..=top {
        let one = sqlx::query_as!(
            RawOutbox,
            r#"SELECT local_id AS "local_id!", stream, record_id, kind, content, created_at_ms,
                task_id, offline_at_create, attempted_at_ms, offline_sent, state, reject_code,
                rejected_at_ms FROM outbox WHERE local_id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await;
        match one {
            Ok(Some(raw)) => match row_of(raw) {
                Ok(row) => rows.push(row),
                Err(_) => unrecovered.push(id),
            },
            Ok(None) => {}
            Err(_) => unrecovered.push(id),
        }
    }
    Some(Rescued {
        rows,
        journal: Vec::new(),
        unrecovered,
        unreadable: false,
    })
}

async fn load_journal_saved(pool: &sqlx::SqlitePool) -> Vec<JournalSaved> {
    let listed = sqlx::query_as::<_, (i64, Option<String>, Option<i64>, Option<String>)>(
        "SELECT local_id, local_detail, turn, effect FROM outbox ORDER BY local_id",
    )
    .fetch_all(pool)
    .await;
    match listed {
        Ok(rows) => rows
            .into_iter()
            .map(|(local_id, local_detail, turn, effect)| JournalSaved {
                local_id,
                local_detail,
                turn,
                effect,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}
