#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("stored value cannot be read: {0}")]
    BadValue(String),
    #[error("records are not contiguous from 1: {0}")]
    NotContiguous(String),
}
