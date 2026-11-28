use anybot_domain as _;
use anybot_foundation as _;
use anybot_protocol as _;
use anybot_store_client::Store;
use serde_json as _;
use sqlx::Row as _;
use thiserror as _;

#[tokio::test]
async fn sqlite_fts5_trigram_available() -> Result<(), String> {
    let dir = test_harness::test_dir("sqlite_fts5_trigram_probe")?;
    let opened = Store::open(&dir, 1)
        .await
        .map_err(|e| format!("open failed: {e}"))?;
    let pool = opened.store.pool();
    let version: String = sqlx::query("SELECT sqlite_version() AS v")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?
        .get("v");
    check_trigram(pool).await?;
    check_unicode61_bm25(pool).await?;
    let mut stdout = std::io::stdout();
    let _w = std::io::Write::write_fmt(&mut stdout, format_args!("sqlite_version={version}\n"));
    opened.store.close().await;
    Ok(())
}

async fn check_trigram(pool: &sqlx::SqlitePool) -> Result<(), String> {
    sqlx::query("CREATE VIRTUAL TABLE probe_trigram USING fts5(content, tokenize='trigram')")
        .execute(pool)
        .await
        .map_err(|e| format!("fts5 trigram unsupported: {e}"))?;
    let cjk = "\u{6e2c}\u{8a66}\u{7528}\u{7684}\u{6587}\u{5b57} log/app/main.rs";
    let other = "\u{5225}\u{7684}\u{5167}\u{5bb9} src/lib.rs";
    sqlx::query("INSERT INTO probe_trigram(content) VALUES (?1), (?2)")
        .bind(cjk)
        .bind(other)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    let path_hits: i64 = sqlx::query(
        "SELECT count(*) AS c FROM probe_trigram WHERE probe_trigram MATCH '\"log/app\"'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?
    .get("c");
    assert_eq!(path_hits, 1);
    let cjk_query = "\"\u{6e2c}\u{8a66}\u{7528}\"";
    let cjk_hits: i64 =
        sqlx::query("SELECT count(*) AS c FROM probe_trigram WHERE probe_trigram MATCH ?1")
            .bind(cjk_query)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?
            .get("c");
    assert_eq!(cjk_hits, 1);
    Ok(())
}

async fn check_unicode61_bm25(pool: &sqlx::SqlitePool) -> Result<(), String> {
    sqlx::query("CREATE VIRTUAL TABLE probe_terms USING fts5(terms, tokenize='unicode61')")
        .execute(pool)
        .await
        .map_err(|e| format!("fts5 unicode61 unsupported: {e}"))?;
    let row_a = "\u{559c} \u{6b61} \u{559c}\u{6b61} \u{8c93} \u{6b61}\u{8c93} cat";
    let row_b = "\u{559c} \u{8c93} dog";
    sqlx::query("INSERT INTO probe_terms(terms) VALUES (?1), (?2)")
        .bind(row_a)
        .bind(row_b)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    let query = "\u{559c}\u{6b61} \u{8c93}";
    let rows = sqlx::query(
        "SELECT rowid, bm25(probe_terms) AS score FROM probe_terms WHERE probe_terms MATCH ?1 ORDER BY score",
    )
    .bind(query)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    assert_eq!(rows.len(), 1);
    let best: i64 = rows.first().ok_or("missing match")?.get("rowid");
    assert_eq!(best, 1);
    let score: f64 = rows.first().ok_or("missing score")?.get("score");
    let mut stdout = std::io::stdout();
    let _w =
        std::io::Write::write_fmt(&mut stdout, format_args!("bm25_top={best} score={score}\n"));
    Ok(())
}
