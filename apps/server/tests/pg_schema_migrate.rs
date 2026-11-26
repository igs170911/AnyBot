use serde_json as _;

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test as _;
use anybot_server::{ensure_schema, min_applied_version, open_url, product_migrations};
use anybot_store_server::SchemaError;
use test_harness::launch::run_program_stdin;
use test_harness::sleep_real;

use anybot_app_test::{ServerProgram, db_start};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

#[tokio::test]
async fn schema_marker_recognizes_foreign_data() -> Result<(), String> {
    let db = db_start("schema_marker_foreign").await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    sqlx::query("DROP TABLE schema_marker")
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("CREATE TABLE notes (id BIGINT PRIMARY KEY)")
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let mut migrator = product_migrations();
    let result = ensure_schema(&pool, &mut migrator).await;
    match result {
        Err(SchemaError::ForeignData { tables }) => {
            assert!(
                tables.iter().any(|name| name == "notes"),
                "tables: {tables:?}"
            );
        }
        other => return Err(format!("foreign marker accepted: {other:?}")),
    }
    let _stop = db.stop().await;
    Ok(())
}

#[tokio::test]
async fn concurrent_migrations_apply_once() -> Result<(), String> {
    let db = db_start("migrate_concurrent").await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    let mut left = product_migrations();
    let expected = left.migrations.len();
    let left_pool = pool.clone();
    let left = tokio::spawn(async move { ensure_schema(&left_pool, &mut left).await });
    let right_pool = pool.clone();
    let right = tokio::spawn(async move {
        let mut second = product_migrations();
        ensure_schema(&right_pool, &mut second).await
    });
    let first = left.await.map_err(|e| e.to_string())?;
    let second = right.await.map_err(|e| e.to_string())?;
    first.map_err(|e| e.to_string())?;
    second.map_err(|e| e.to_string())?;
    let rows = sqlx::query("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), expected, "applied rows doubled under the lock");
    let low = min_applied_version(&pool)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(low, 1, "lowest applied version");
    let _stop = db.stop().await;
    Ok(())
}

#[tokio::test]
async fn cli_reset_stops_on_schema_version_mismatch() -> Result<(), String> {
    let db = db_start("cli_reset_mismatch").await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, installed_on, \
         success, checksum, execution_time) \
         VALUES (99999999, 'future', now(), true, '\\x00000000000000000000000000000000', 0)",
    )
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    let program = server_bin()
        .program(vec![
            "--bootstrap".to_owned(),
            db.bootstrap.to_string_lossy().to_string(),
            "reset-password".to_owned(),
            "alice".to_owned(),
        ])
        .with_timeout(std::time::Duration::from_secs(15));
    let outcome = run_program_stdin(&program, &db.dir, "unusedpassword\nunusedpassword\n")?;
    sleep_real(std::time::Duration::from_millis(10)).await;
    assert_eq!(outcome.code, 3, "stdout: {}", outcome.stdout);
    assert!(
        outcome.stderr.contains("schema newer than"),
        "stderr: {}",
        outcome.stderr
    );
    let _stop = db.stop().await;
    Ok(())
}
