use aes_gcm as _;
use anybot_foundation as _;
use anybot_store_server as _;
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
use tracing as _;
use tracing_subscriber as _;
use url as _;

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use test_harness::PgInstance;
use test_harness::postgres::{pg_start, pg_stop};
use test_harness::realtime::timeout_real;
use test_harness::scratch::test_dir;

const START_BUDGET: Duration = Duration::from_secs(240);

async fn start_pg(label: &str) -> Result<PgInstance, String> {
    let base = test_dir(label)?;
    timeout_real(pg_start(&base), START_BUDGET).await?
}

async fn open_pool(instance: &PgInstance, database: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(20))
        .connect(&instance.url_for(database))
        .await
        .map_err(|e| format!("connect pool failed: {e}"))
}

async fn insert(pool: &PgPool, name: &str) -> Result<i64, String> {
    let row = sqlx::query("INSERT INTO items (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("insert {name} failed: {e}"))?;
    Ok(row.get::<i64, _>("id"))
}

async fn names(pool: &PgPool) -> Result<Vec<String>, String> {
    let rows = sqlx::query("SELECT name FROM items ORDER BY id")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("select failed: {e}"))?;
    Ok(rows.iter().map(|r| r.get::<String, _>("name")).collect())
}

async fn seeded(instance: &PgInstance, database: &str) -> Result<PgPool, String> {
    instance.create_database(database).await?;
    let pool = open_pool(instance, database).await?;
    sqlx::query("CREATE TABLE items (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .map_err(|e| format!("create table failed: {e}"))?;
    insert(&pool, "a").await?;
    insert(&pool, "b").await?;
    Ok(pool)
}

#[tokio::test]
async fn postgres_restore_goes_back_to_the_snapshot_rows_and_sequence() {
    let instance = start_pg("pg-snapshot-restore").await.unwrap();
    let pool = seeded(&instance, "main").await.unwrap();
    instance
        .snapshot_database("main", "main_snap")
        .await
        .unwrap();
    assert_eq!(insert(&pool, "c").await.unwrap(), 3);
    assert_eq!(insert(&pool, "d").await.unwrap(), 4);
    pool.close().await;
    instance
        .restore_database("main", "main_snap")
        .await
        .unwrap();
    let restored = open_pool(&instance, "main").await.unwrap();
    assert_eq!(names(&restored).await.unwrap(), vec!["a", "b"]);
    assert_eq!(insert(&restored, "x").await.unwrap(), 3);
    restored.close().await;
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_snapshot_works_with_a_live_pool_and_restore_can_repeat() {
    let instance = start_pg("pg-snapshot-live").await.unwrap();
    let pool = seeded(&instance, "main").await.unwrap();
    instance
        .snapshot_database("main", "main_snap")
        .await
        .unwrap();
    insert(&pool, "c").await.unwrap();
    for round in 0..2 {
        instance
            .restore_database("main", "main_snap")
            .await
            .unwrap();
        assert_eq!(
            names(&pool).await.unwrap(),
            vec!["a", "b"],
            "restore round {round} through the old pool"
        );
        insert(&pool, "again").await.unwrap();
    }
    pool.close().await;
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_taking_a_snapshot_again_replaces_the_older_one() {
    let instance = start_pg("pg-snapshot-replace").await.unwrap();
    let pool = seeded(&instance, "main").await.unwrap();
    instance
        .snapshot_database("main", "main_snap")
        .await
        .unwrap();
    insert(&pool, "c").await.unwrap();
    instance
        .snapshot_database("main", "main_snap")
        .await
        .unwrap();
    insert(&pool, "d").await.unwrap();
    instance
        .restore_database("main", "main_snap")
        .await
        .unwrap();
    assert_eq!(names(&pool).await.unwrap(), vec!["a", "b", "c"]);
    pool.close().await;
    pg_stop(&instance).await.unwrap();
}

#[tokio::test]
async fn postgres_restore_leaves_other_databases_alone() {
    let instance = start_pg("pg-snapshot-others").await.unwrap();
    let main = seeded(&instance, "main").await.unwrap();
    let other = seeded(&instance, "other").await.unwrap();
    instance
        .snapshot_database("main", "main_snap")
        .await
        .unwrap();
    insert(&main, "c").await.unwrap();
    insert(&other, "o").await.unwrap();
    instance
        .restore_database("main", "main_snap")
        .await
        .unwrap();
    assert_eq!(names(&other).await.unwrap(), vec!["a", "b", "o"]);
    assert_eq!(names(&main).await.unwrap(), vec!["a", "b"]);
    main.close().await;
    other.close().await;
    pg_stop(&instance).await.unwrap();
}
