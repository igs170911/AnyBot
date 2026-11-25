use std::io::Read;
use std::path::Path;

use url::Url;

use crate::error::{
    CODE_BOOTSTRAP_CREDENTIALS_MISSING, CODE_BOOTSTRAP_DATABASE_MISSING,
    CODE_BOOTSTRAP_HOST_MISSING, CODE_BOOTSTRAP_NOT_FOUND, CODE_BOOTSTRAP_PERMISSIONS_READ_FAILED,
    CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN, CODE_BOOTSTRAP_READ_FAILED, CODE_BOOTSTRAP_URL_MALFORMED,
    CODE_BOOTSTRAP_URL_SCHEME, FoundationError, MessageRef,
};
use crate::secret::Secret;

pub const DEFAULT_POSTGRES_PORT: u16 = 5432;

#[derive(Debug)]
pub struct ServerBootstrap {
    pub user: String,
    pub password: Secret,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub query: String,
}

impl ServerBootstrap {
    #[must_use]
    pub fn describe_db(&self) -> String {
        format!("{}/{}", self.host, self.database)
    }

    pub fn read(path: &Path) -> Result<Self, FoundationError> {
        let shown = path.display().to_string();
        let mut file = std::fs::File::open(path).map_err(|e| {
            FoundationError::BootstrapNotFound(
                MessageRef::new(CODE_BOOTSTRAP_NOT_FOUND)
                    .with_text("path", &shown)
                    .with_text("reason", &e.to_string()),
            )
        })?;
        let meta = file.metadata().map_err(|e| {
            FoundationError::BootstrapPermissions(
                MessageRef::new(CODE_BOOTSTRAP_PERMISSIONS_READ_FAILED)
                    .with_text("path", &shown)
                    .with_text("reason", &e.to_string()),
            )
        })?;
        check_permissions(path, &meta)?;
        let mut text = String::new();
        file.read_to_string(&mut text).map_err(|e| {
            FoundationError::BootstrapNotFound(
                MessageRef::new(CODE_BOOTSTRAP_READ_FAILED)
                    .with_text("path", &shown)
                    .with_text("reason", &e.to_string()),
            )
        })?;
        parse_connection(text.trim())
    }
}

fn check_permissions(path: &Path, meta: &std::fs::Metadata) -> Result<(), FoundationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = meta.mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(FoundationError::BootstrapPermissions(
                MessageRef::new(CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN)
                    .with_text("path", &path.display().to_string())
                    .with_text("mode", &format!("{mode:o}")),
            ));
        }
    }
    Ok(())
}

pub fn parse_connection(text: &str) -> Result<ServerBootstrap, FoundationError> {
    let url = Url::parse(text).map_err(|_| {
        FoundationError::BootstrapMalformed(MessageRef::new(CODE_BOOTSTRAP_URL_MALFORMED))
    })?;
    if url.scheme() != "postgres" && url.scheme() != "postgresql" {
        return Err(FoundationError::BootstrapMalformed(MessageRef::new(
            CODE_BOOTSTRAP_URL_SCHEME,
        )));
    }
    if url.username().is_empty() && url.password().is_none() {
        return Err(FoundationError::BootstrapMalformed(MessageRef::new(
            CODE_BOOTSTRAP_CREDENTIALS_MISSING,
        )));
    }
    let user = percent_decode(url.username());
    let password = url.password().map(percent_decode).unwrap_or_default();
    if user.is_empty() || password.is_empty() {
        return Err(FoundationError::BootstrapMalformed(MessageRef::new(
            CODE_BOOTSTRAP_CREDENTIALS_MISSING,
        )));
    }
    let host = url
        .host_str()
        .ok_or(FoundationError::BootstrapMalformed(MessageRef::new(
            CODE_BOOTSTRAP_HOST_MISSING,
        )))?
        .to_owned();
    let port = url.port().unwrap_or(DEFAULT_POSTGRES_PORT);
    let database = percent_decode(url.path().trim_start_matches('/'));
    if database.is_empty() {
        return Err(FoundationError::BootstrapMalformed(MessageRef::new(
            CODE_BOOTSTRAP_DATABASE_MISSING,
        )));
    }
    Ok(ServerBootstrap {
        user,
        password: Secret::from_untrusted(password),
        host,
        port,
        database,
        query: url.query().unwrap_or("").to_owned(),
    })
}

fn percent_decode(part: &str) -> String {
    percent_encoding::percent_decode_str(part)
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
