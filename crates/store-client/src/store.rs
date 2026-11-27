use std::path::{Path, PathBuf};

pub type Pool = sqlx::SqlitePool;

#[derive(Debug, Clone)]
pub struct Store {
    pub(crate) pool: Pool,
    pub(crate) path: PathBuf,
}

impl Store {
    pub(crate) fn from_parts(pool: Pool, path: PathBuf) -> Self {
        Store { pool, path }
    }

    #[must_use]
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
