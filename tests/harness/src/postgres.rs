use std::path::{Path, PathBuf};
use std::time::Duration;

use postgresql_embedded::{PostgreSQL, Settings, VersionReq};

pub const PG_VERSION_EXACT: &str = "=17.6.0";
pub const CODE_PG_START: &str = "pg-start";

#[derive(Debug, Clone)]
pub struct PgInstance {
    server: PostgreSQL,
    base_dir: PathBuf,
}

impl PgInstance {
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.server.settings().port
    }

    #[must_use]
    pub fn url_for(&self, database: &str) -> String {
        let settings = self.server.settings();
        format!(
            "postgres://{}:{}@{}:{}/{}",
            settings.username, settings.password, settings.host, settings.port, database
        )
    }

    pub async fn create_database(&self, database: &str) -> Result<(), String> {
        let server = clone_server(self);
        server
            .create_database(database)
            .await
            .map_err(|e| format!("create database {database} failed: {e}"))
    }

    pub async fn drop_database(&self, database: &str) -> Result<(), String> {
        let server = clone_server(self);
        server
            .drop_database(database)
            .await
            .map_err(|e| format!("drop database {database} failed: {e}"))
    }
}

fn clone_server(instance: &PgInstance) -> PostgreSQL {
    PostgreSQL::new(instance.server.settings().clone())
}

pub async fn pg_start(base_dir: &Path) -> Result<PgInstance, String> {
    let data_dir = base_dir.join("data");
    let socket_dir = base_dir.join("sock");
    let installation_dir = base_dir.join("install");
    for dir in [&data_dir, &socket_dir, &installation_dir] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("create {} failed: {e}", dir.display()))?;
    }
    let settings = Settings {
        version: VersionReq::parse(PG_VERSION_EXACT)
            .map_err(|e| format!("bad pinned postgres version: {e}"))?,
        host: "127.0.0.1".to_owned(),
        username: "anybot".to_owned(),
        password: pg_generated_password(),
        data_dir,
        socket_dir: Some(socket_dir),
        password_file: base_dir.join("pw"),
        installation_dir,
        temporary: true,
        trust_installation_dir: false,
        timeout: Some(Duration::from_secs(120)),
        ..Default::default()
    };
    let mut server = PostgreSQL::new(settings);
    server
        .setup()
        .await
        .map_err(|e| format!("postgres setup failed: {e}"))?;
    server
        .start()
        .await
        .map_err(|e| format!("postgres start failed: {e}"))?;
    Ok(PgInstance {
        server,
        base_dir: base_dir.to_path_buf(),
    })
}

pub async fn pg_stop(instance: &PgInstance) -> Result<(), String> {
    instance
        .server
        .stop()
        .await
        .map_err(|e| format!("postgres stop failed: {e}"))?;
    Ok(())
}

fn pg_generated_password() -> String {
    let pid = std::process::id();
    let nanos = std::time::UNIX_EPOCH
        .elapsed()
        .map_or(0, |d| d.as_nanos() % 1_000_000_000);
    format!("pgtest-{pid}-{nanos}")
}

#[must_use]
pub fn pg_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("migrations")
        .join("server")
}

#[must_use]
pub fn pg_ready(_base_dir: &Path, port: u16) -> bool {
    std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
}
