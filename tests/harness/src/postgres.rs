use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::net::TcpStream;

use postgresql_embedded::{PostgreSQL, Settings, VersionReq};

pub const PG_VERSION_EXACT: &str = "=17.6.0";
#[derive(Debug, Clone)]
pub struct PgInstance {
    server: PostgreSQL,
    base_dir: PathBuf,
    bound_port: u16,
}

impl PgInstance {
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.bound_port
    }

    #[must_use]
    pub fn user(&self) -> String {
        self.server.settings().username.clone()
    }

    #[must_use]
    pub fn password(&self) -> anybot_foundation::Secret {
        anybot_foundation::Secret::from_untrusted(self.server.settings().password.clone())
    }

    #[must_use]
    pub fn url_for(&self, database: &str) -> String {
        let settings = self.server.settings();
        format!(
            "postgres://{}:{}@{}:{}/{}",
            settings.username,
            settings.password,
            settings.host,
            self.port(),
            database
        )
    }

    pub async fn create_database(&self, database: &str) -> Result<(), String> {
        let pool = self.admin_pool().await?;
        let quoted = database.replace('"', "");
        let result = sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE \"{quoted}\"")))
            .execute(&pool)
            .await;
        pool.close().await;
        result
            .map(|_rows| ())
            .map_err(|e| format!("create database {database} failed: {e}"))
    }

    pub async fn drop_database(&self, database: &str) -> Result<(), String> {
        let pool = self.admin_pool().await?;
        let quoted = database.replace('"', "");
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS \"{quoted}\""
        )))
        .execute(&pool)
        .await;
        pool.close().await;
        result
            .map(|_rows| ())
            .map_err(|e| format!("drop database {database} failed: {e}"))
    }

    pub async fn pause(&self) -> Result<(), String> {
        self.server
            .stop()
            .await
            .map_err(|e| format!("postgres pause failed: {e}"))?;
        Ok(())
    }

    pub async fn resume(&mut self) -> Result<(), String> {
        self.server
            .start()
            .await
            .map_err(|e| format!("postgres resume failed: {e}"))?;
        let data_dir = self.base_dir.join("data");
        let confirmed = resolve_bound_port(&data_dir, self.bound_port).await?;
        if confirmed != self.bound_port {
            return Err(format!(
                "postgres resumed on port {confirmed}, wanted {}",
                self.bound_port
            ));
        }
        Ok(())
    }

    async fn admin_pool(&self) -> Result<sqlx::PgPool, String> {
        let settings = self.server.settings();
        let url = format!(
            "postgres://{}:{}@{}:{}/postgres",
            settings.username, settings.password, settings.host, settings.port
        );
        sqlx::PgPool::connect(url.as_str())
            .await
            .map_err(|e| format!("admin pool connect failed: {e}"))
    }
}

#[must_use]
pub fn pg_install_cache_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("pg-install")
}

pub async fn pg_start(base_dir: &Path) -> Result<PgInstance, String> {
    let data_dir = base_dir.join("data");
    let installation_dir = pg_install_cache_dir();
    for dir in [&data_dir, &installation_dir] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("create {} failed: {e}", dir.display()))?;
    }
    let settings = Settings {
        version: VersionReq::parse(PG_VERSION_EXACT)
            .map_err(|e| format!("bad pinned postgres version: {e}"))?,
        host: "127.0.0.1".to_owned(),
        username: "postgres".to_owned(),
        password: pg_generated_password(),
        port: 0,
        data_dir: data_dir.clone(),
        socket_dir: Some(base_dir.to_path_buf()),
        password_file: base_dir.join("pw"),
        installation_dir,
        temporary: false,
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
    let bound_port = resolve_bound_port(&data_dir, server.settings().port).await?;
    Ok(PgInstance {
        server,
        base_dir: base_dir.to_path_buf(),
        bound_port,
    })
}

async fn resolve_bound_port(data_dir: &Path, wanted: u16) -> Result<u16, String> {
    for _tick in 0..150 {
        let probe = pid_file_port(data_dir).unwrap_or(wanted);
        if TcpStream::connect(("127.0.0.1", probe)).await.is_ok() {
            return Ok(probe);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(format!("postgres port {wanted} never answered"))
}

fn pid_file_port(data_dir: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(data_dir.join("postmaster.pid")).ok()?;
    let line = text.lines().nth(3)?;
    line.trim().parse::<u16>().ok()
}

pub async fn pg_stop(instance: &PgInstance) -> Result<(), String> {
    instance
        .server
        .stop()
        .await
        .map_err(|e| format!("postgres stop failed: {e}"))?;
    let _removed = std::fs::remove_dir_all(instance.base_dir()).is_ok();
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

#[must_use]
pub fn pg_socket_path(base_dir: &Path, port: u16) -> PathBuf {
    base_dir.join(format!(".s.PGSQL.{port}"))
}

pub fn migration_fixture(product_dir: &Path, test: &str, extra: &str) -> Result<PathBuf, String> {
    let dir = crate::scratch::test_dir(test)?.join("migrations");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create fixture dir failed: {e}"))?;
    for entry in std::fs::read_dir(product_dir)
        .map_err(|e| format!("read product migrations failed: {e}"))?
    {
        let path = entry
            .map_err(|e| format!("fixture entry failed: {e}"))?
            .path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        std::fs::copy(&path, dir.join(&name)).map_err(|e| format!("fixture copy failed: {e}"))?;
    }
    let sql = if extra.contains("broken") {
        "NOT SQL AT ALL;\n".to_owned()
    } else {
        "CREATE TABLE IF NOT EXISTS fixture_extra (id INTEGER PRIMARY KEY);\n".to_owned()
    };
    std::fs::write(dir.join(extra), sql).map_err(|e| format!("fixture write failed: {e}"))?;
    Ok(dir)
}
