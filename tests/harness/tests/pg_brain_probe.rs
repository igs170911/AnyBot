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

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;

use test_harness::postgres::{pg_start, pg_stop};
use test_harness::realtime::timeout_real;
use test_harness::scratch::test_dir;

const START_BUDGET: Duration = Duration::from_secs(240);

async fn open_pool(instance: &test_harness::PgInstance, database: &str) -> Result<PgPool, String> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(20))
        .connect(&instance.url_for(database))
        .await
        .map_err(|e| format!("connect pool failed: {e}"))
}

fn log_line(line: &str) {
    let mut stdout = std::io::stdout();
    let _w = std::io::Write::write_fmt(&mut stdout, format_args!("{line}\n"));
}

async fn report_extensions(pool: &PgPool) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT name, default_version, COALESCE(installed_version, 'none') AS installed \
         FROM pg_available_extensions \
         WHERE name IN ('pg_trgm', 'vector', 'pg_bigm') ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut has_trgm = false;
    for row in &rows {
        let name: String = row.get("name");
        let ver: String = row.get("default_version");
        let inst: String = row.get("installed");
        log_line(&format!("ext={name} default={ver} installed={inst}"));
        if name == "pg_trgm" {
            has_trgm = true;
        }
    }
    assert!(
        has_trgm,
        "pg_trgm extension is missing from embedded PostgreSQL"
    );
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .execute(pool)
        .await
        .map_err(|e| format!("CREATE EXTENSION pg_trgm failed: {e}"))?;
    Ok(())
}

async fn check_locale_and_tsvector(pool: &PgPool, label: &str) -> Result<String, String> {
    let db_row = sqlx::query(
        "SELECT pg_encoding_to_char(encoding) AS enc, datctype, datcollate, \
         datlocprovider::text AS provider, COALESCE(datlocale, '') AS datlocale \
         FROM pg_database WHERE datname = current_database()",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    let enc: String = db_row.get("enc");
    let datctype: String = db_row.get("datctype");
    let datcollate: String = db_row.get("datcollate");
    let provider: String = db_row.get("provider");
    let locale: String = db_row.get("datlocale");
    log_line(&format!(
        "{label}: enc={enc} datctype={datctype} datcollate={datcollate} provider={provider} locale={locale}"
    ));
    let sample = "\u{53f0}\u{7063} \u{53f0} \u{7063} ERR_42 a/b/c";
    let tsv: String = sqlx::query("SELECT to_tsvector('simple', $1)::text AS t")
        .bind(sample)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get("t");
    let cjk_trgm: String = sqlx::query("SELECT show_trgm($1)::text AS g")
        .bind("\u{6e2c}\u{8a66}\u{7528}")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get("g");
    log_line(&format!("{label}: tsvector={tsv} cjk_trgm={cjk_trgm}"));
    assert!(tsv.contains("\u{53f0}\u{7063}"));
    assert!(tsv.contains("err"));
    Ok(cjk_trgm)
}

async fn setup_probe_table(pool: &PgPool, rows: i64) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE probe_mem (id SERIAL PRIMARY KEY, content TEXT NOT NULL, terms TSVECTOR NOT NULL)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query("CREATE INDEX idx_probe_trgm ON probe_mem USING gin (content gin_trgm_ops)")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("CREATE INDEX idx_probe_terms ON probe_mem USING gin (terms)")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO probe_mem (content, terms) \
         SELECT 'filler ' || g || ' padding text line for index test', \
         to_tsvector('simple', 'filler ' || g) FROM generate_series(1, $1) AS g",
    )
    .bind(rows)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query("INSERT INTO probe_mem (content, terms) VALUES ($1, to_tsvector('simple', $2))")
        .bind("\u{6e2c}\u{8a66}\u{7528}\u{7684}\u{6587}\u{5b57} log/app/main.rs")
        .bind("\u{6e2c}\u{8a66} \u{8a66}\u{7528} log app")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("ANALYZE probe_mem")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn check_plan(pool: &PgPool, sql: &str, label: &str) -> Result<String, String> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("EXPLAIN {sql}")))
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let plan: Vec<String> = rows.iter().map(|r| r.get::<String, _>(0)).collect();
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let idx_rows = sqlx::query(sqlx::AssertSqlSafe(format!("EXPLAIN {sql}")))
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    let idx_plan: Vec<String> = idx_rows.iter().map(|r| r.get::<String, _>(0)).collect();
    let idx_joined = idx_plan.join(" | ");
    log_line(&format!("explain_{label}={}", plan.join(" | ")));
    log_line(&format!("explain_{label}_noseq={idx_joined}"));
    Ok(idx_joined)
}

async fn check_trgm_explain(pool: &PgPool, label: &str, rows: i64) -> Result<(), String> {
    setup_probe_table(pool, rows).await?;
    let p1 = check_plan(
        pool,
        "SELECT id FROM probe_mem WHERE content LIKE '%log/app%'",
        &format!("{label}_path_trgm"),
    )
    .await?;
    let p2 = check_plan(
        pool,
        "SELECT id FROM probe_mem WHERE content LIKE '%\u{6e2c}\u{8a66}\u{7528}%'",
        &format!("{label}_cjk_trgm"),
    )
    .await?;
    let p3 = check_plan(
        pool,
        "SELECT id FROM probe_mem WHERE terms @@ to_tsquery('simple', '\u{6e2c}\u{8a66}')",
        &format!("{label}_cjk_gin"),
    )
    .await?;
    if label == "loc_builtin_c_utf8" {
        assert!(p1.contains("Index"), "path_trgm plan: {p1}");
        assert!(p2.contains("Index"), "cjk_trgm plan: {p2}");
        assert!(p3.contains("Index"), "cjk_gin plan: {p3}");
    }
    Ok(())
}

async fn probe_utf8_db(instance: &test_harness::PgInstance, pool: &PgPool) -> Result<(), String> {
    let candidates = [
        (
            "loc_c_only",
            "CREATE DATABASE loc_c_only TEMPLATE template0 LC_CTYPE 'C' LC_COLLATE 'C'",
            5_000_i64,
        ),
        (
            "loc_builtin_c_utf8",
            "CREATE DATABASE loc_builtin_c_utf8 TEMPLATE template0 LOCALE_PROVIDER builtin BUILTIN_LOCALE 'C.UTF-8'",
            25_000_i64,
        ),
    ];
    for (name, sql, rows) in candidates {
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(pool)
            .await
            .map_err(|e| format!("{name} create failed: {e}"))?;
        let p = open_pool(instance, name).await?;
        report_extensions(&p).await?;
        let cjk_trgm = check_locale_and_tsvector(&p, name).await?;
        if name == "loc_builtin_c_utf8" {
            assert_ne!(cjk_trgm, "{}", "UTF-8 locale must extract CJK trigrams");
        }
        check_trgm_explain(&p, name, rows).await?;
        p.close().await;
    }
    Ok(())
}

#[tokio::test]
async fn pg_brain_extensions_report() -> Result<(), String> {
    let base = test_dir("pg-brain-probe")?;
    let instance = timeout_real(pg_start(&base), START_BUDGET).await??;
    instance.create_database("brain_probe").await?;
    let pool = open_pool(&instance, "brain_probe").await?;
    report_extensions(&pool).await?;
    check_locale_and_tsvector(&pool, "default").await?;
    probe_utf8_db(&instance, &pool).await?;
    pool.close().await;
    pg_stop(&instance).await?;
    Ok(())
}
