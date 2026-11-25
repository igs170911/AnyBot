use thiserror::Error;

#[derive(Error, Debug)]
pub enum FoundationError {
    #[error("{0}")]
    ConfigMissing(&'static str),

    #[error("{0}")]
    BootstrapNotFound(String),

    #[error("{0}")]
    BootstrapPermissions(String),

    #[error("{0}")]
    BootstrapMalformed(String),

    #[error("{0}")]
    IdParse(String),

    #[error("{0}")]
    DataDirMissing(String),
}
