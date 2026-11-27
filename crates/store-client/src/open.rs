use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::ConnectOptions as _;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::error::StoreError;
use crate::recover::{Recovery, RecoveryReason};
use crate::store::{Pool, Store};

pub const DATABASE_FILE: &str = "replica.sqlite3";

pub(crate) static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/client");

pub struct OpenOutcome {
    pub store: Store,
    pub recovery: Option<Recovery>,
}

impl Store {
    pub async fn open(data_dir: &Path, stamp_ms: i64) -> Result<OpenOutcome, StoreError> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join(DATABASE_FILE);
        if !path.exists() {
            let store = connect_migrated(&path).await?;
            return Ok(OpenOutcome {
                store,
                recovery: None,
            });
        }
        if let Some(reason) = crate::recover_read::probe(&path).await? {
            return crate::recover::recover(data_dir, &path, reason, stamp_ms).await;
        }
        match connect_migrated(&path).await {
            Ok(store) => Ok(OpenOutcome {
                store,
                recovery: None,
            }),
            Err(StoreError::Io(err)) => Err(StoreError::Io(err)),
            Err(other) => {
                let reason = RecoveryReason::Damaged(other.to_string());
                crate::recover::recover(data_dir, &path, reason, stamp_ms).await
            }
        }
    }
}

pub(crate) fn writable_options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(Duration::from_secs(10))
}

pub(crate) async fn connect_migrated(path: &Path) -> Result<Store, StoreError> {
    let pool = connect_pool(path).await?;
    if let Err(err) = MIGRATOR.run(&pool).await {
        pool.close().await;
        return Err(StoreError::Migrate(err));
    }
    Ok(Store::from_parts(pool, path.to_path_buf()))
}

pub(crate) async fn connect_pool(path: &Path) -> Result<Pool, StoreError> {
    let options = writable_options(path).disable_statement_logging();
    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(StoreError::Database)
}

pub(crate) fn immutable_options(path: &Path) -> Result<SqliteConnectOptions, StoreError> {
    let text = format!("sqlite://{}", path.display());
    let options = SqliteConnectOptions::from_str(&text).map_err(StoreError::Database)?;
    Ok(options
        .read_only(true)
        .immutable(true)
        .disable_statement_logging())
}
