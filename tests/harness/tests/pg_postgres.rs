use aes_gcm as _;
use anybot_foundation as _;
use axum as _;
use base64 as _;
use bytes as _;
use getrandom as _;
use hkdf as _;
use libc as _;
use p256 as _;
use postgresql_embedded as _;
use rcgen as _;
use reqwest as _;
use rustls as _;
use rustls_pki_types as _;
use sad_web_push as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use socket2 as _;
use time as _;
use tokio_rustls as _;
use tokio_stream as _;
use toml as _;
use url as _;

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use test_harness::ports::port_is_free;
use test_harness::postgres::{
    pg_install_cache_dir, pg_migrations_dir, pg_ready, pg_socket_path, pg_start, pg_stop,
};
use test_harness::realtime::timeout_real;
use test_harness::scratch::test_dir;

const START_BUDGET: Duration = Duration::from_secs(240);

async fn start_pg(label: &str) -> Result<test_harness::PgInstance, String> {
    let base = test_dir(label)?;
    timeout_real(pg_start(&base), START_BUDGET).await?
}

async fn open_pool(instance: &test_harness::PgInstance, database: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(20))
        .connect(&instance.url_for(database))
        .await
        .map_err(|e| format!("connect pool failed: {e}"))
}

#[tokio::test]
async fn postgres_migration_insert_and_read_back() {
    let instance = start_pg("pg-roundtrip").await.unwrap();
    instance.create_database("roundtrip").await.unwrap();
    let pool = open_pool(&instance, "roundtrip").await.unwrap();
    let migrator = sqlx::migrate::Migrator::new(pg_migrations_dir())
        .await
        .unwrap();
    migrator.run(&pool).await.unwrap();
    sqlx::query("INSERT INTO widgets (id, name) VALUES ($1, $2)")
        .bind(7)
        .bind("gear")
        .execute(&pool)
        .await
        .unwrap();
    let row = sqlx::query("SELECT id, name FROM widgets WHERE id = $1")
        .bind(7)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i32, _>("id"), 7);
    assert_eq!(row.get::<String, _>("name"), "gear");
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_two_databases_do_not_see_each_other() {
    let instance = start_pg("pg-isolation").await.unwrap();
    instance.create_database("alpha").await.unwrap();
    instance.create_database("beta").await.unwrap();
    let pool_alpha = open_pool(&instance, "alpha").await.unwrap();
    let pool_beta = open_pool(&instance, "beta").await.unwrap();
    let migrator = sqlx::migrate::Migrator::new(pg_migrations_dir())
        .await
        .unwrap();
    migrator.run(&pool_alpha).await.unwrap();
    sqlx::query("INSERT INTO widgets (id, name) VALUES ($1, $2)")
        .bind(1)
        .bind("alpha-only")
        .execute(&pool_alpha)
        .await
        .unwrap();
    let count: i64 = sqlx::query("SELECT count(*) AS c FROM widgets")
        .fetch_one(&pool_alpha)
        .await
        .unwrap()
        .get("c");
    assert_eq!(count, 1);
    let beta_has_table = sqlx::query("SELECT count(*) AS c FROM widgets")
        .fetch_one(&pool_beta)
        .await;
    assert!(beta_has_table.is_err());
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_stop_releases_port_and_process() {
    let instance = start_pg("pg-stop").await.unwrap();
    let port = instance.port();
    assert!(pg_ready(instance.base_dir(), port));
    instance.create_database("probe").await.unwrap();
    let pool = open_pool(&instance, "probe").await.unwrap();
    let row = sqlx::query("SELECT 1 AS one")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i32, _>("one"), 1);
    pool.close().await;
    pg_stop(&instance).await.unwrap();
    assert!(!pg_ready(instance.base_dir(), port));
    assert!(port_is_free(port));
}

#[tokio::test]
async fn postgres_install_cache_is_shared_under_target() {
    let cache = pg_install_cache_dir();
    let resolved = std::path::absolute(&cache).unwrap_or(cache.clone());
    assert!(resolved.is_absolute());
    let tail = resolved.file_name().unwrap();
    assert_eq!(tail, "pg-install");
    let parent = resolved.parent().unwrap().file_name().unwrap();
    assert_eq!(parent, "target");
}

#[tokio::test]
async fn postgres_socket_directory_is_under_target() {
    let instance = start_pg("pg-socket").await.unwrap();
    let socket = pg_socket_path(instance.base_dir(), instance.port());
    let socket_text = socket.display().to_string();
    let mut stdout = std::io::stdout();
    let _written = std::io::Write::write_fmt(
        &mut stdout,
        format_args!("unix socket path: {socket_text}\n"),
    );
    let resolved = std::path::absolute(&socket).unwrap_or(socket.clone());
    assert!(
        resolved.exists(),
        "socket file missing at {}",
        resolved.display()
    );
    assert!(
        resolved.starts_with(test_harness::repo_root().join("target")),
        "socket must live under target/: {}",
        resolved.display()
    );
    let path_text = resolved.to_string_lossy();
    assert!(
        path_text.len() <= 100,
        "macOS unix socket path limit is about 104: got {}",
        path_text.len()
    );
    let tmp_socket = std::path::Path::new("/tmp").join(format!(".s.PGSQL.{}", instance.port()));
    assert!(
        !tmp_socket.exists(),
        "socket leaked to /tmp: {}",
        tmp_socket.display()
    );
    let lock = std::path::Path::new("/tmp").join(format!(".s.PGSQL.{}.lock", instance.port()));
    assert!(
        !lock.exists(),
        "socket lock leaked to /tmp: {}",
        lock.display()
    );
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_dropped_database_disappears() {
    let instance = start_pg("pg-drop").await.unwrap();
    instance.create_database("tempdb").await.unwrap();
    let pool = open_pool(&instance, "tempdb").await.unwrap();
    pool.close().await;
    instance.drop_database("tempdb").await.unwrap();
    let reopened = open_pool(&instance, "tempdb").await;
    assert!(reopened.is_err());
    pg_stop(&instance).await.unwrap();
}
