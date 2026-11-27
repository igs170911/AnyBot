use std::path::{Path, PathBuf};

use crate::error::StoreError;
use crate::open::{OpenOutcome, connect_migrated};
use crate::recover_read::{Rescued, rescue_outbox};
use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryReason {
    Damaged(String),
    NotSqlite(String),
    NewerSchema(String),
}

impl RecoveryReason {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            RecoveryReason::Damaged(_) => "replica-damaged",
            RecoveryReason::NotSqlite(_) => "replica-not-sqlite",
            RecoveryReason::NewerSchema(_) => "replica-newer-schema",
        }
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            RecoveryReason::Damaged(text)
            | RecoveryReason::NotSqlite(text)
            | RecoveryReason::NewerSchema(text) => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    pub reason: RecoveryReason,
    pub moved_to: PathBuf,
    pub rescued_outbox: usize,
    pub unrecovered_outbox: Vec<i64>,
    pub outbox_unreadable: bool,
}

pub(crate) async fn recover(
    data_dir: &Path,
    path: &Path,
    reason: RecoveryReason,
    stamp_ms: i64,
) -> Result<OpenOutcome, StoreError> {
    let rescued = rescue_outbox(data_dir, path, stamp_ms).await;
    let moved_to = move_aside(path, stamp_ms)?;
    let store = connect_migrated(path).await?;
    let count = rescued.rows.len();
    import_rows(&store, &rescued).await?;
    Ok(OpenOutcome {
        store,
        recovery: Some(Recovery {
            reason,
            moved_to,
            rescued_outbox: count,
            unrecovered_outbox: rescued.unrecovered,
            outbox_unreadable: rescued.unreadable,
        }),
    })
}

async fn import_rows(store: &Store, rescued: &Rescued) -> Result<(), StoreError> {
    for row in &rescued.rows {
        store.import_outbox_row(row).await?;
    }
    Ok(())
}

fn move_aside(path: &Path, stamp_ms: i64) -> Result<PathBuf, StoreError> {
    let base = path.to_string_lossy().into_owned();
    let mut counter = 0u32;
    loop {
        let suffix = if counter == 0 {
            format!(".damaged-{stamp_ms}")
        } else {
            format!(".damaged-{stamp_ms}-{counter}")
        };
        let target = PathBuf::from(format!("{base}{suffix}"));
        if !target.exists() && !PathBuf::from(format!("{base}{suffix}-wal")).exists() {
            move_with_sidecars(&base, &suffix)?;
            return Ok(target);
        }
        counter = counter.saturating_add(1);
        if counter > 10_000 {
            return Err(StoreError::BadValue(
                "no free name for the damaged copy".to_owned(),
            ));
        }
    }
}

fn move_with_sidecars(base: &str, suffix: &str) -> Result<(), StoreError> {
    for tail in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{base}{tail}"));
        if from.exists() {
            let to = PathBuf::from(format!("{base}{suffix}{tail}"));
            std::fs::rename(&from, &to)?;
        }
    }
    Ok(())
}
