use serde_json::{Value, json};

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::serve::serve_start_with;
use anybot_app_test::sse::{data_json, event_name};
use anybot_domain::version_release::ReleaseTable;

mod pg_api_streams_sse_shared;
use pg_api_streams_sse_shared::{
    DOLPHIN_TEXT, append, append_body, item, login_bearer, open_account, progress_seq, seed,
};

#[tokio::test]
async fn sse_first_event_is_progress_with_the_max() -> Result<(), String> {
    let served = serve_start_with("sse_progress", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 5).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let block = stream.recv_block().await?;
    assert_eq!(
        event_name(&block).as_deref(),
        Some("progress"),
        "head: {block}"
    );
    let json = data_json(&block)?;
    assert_eq!(progress_seq(&json)?, 5, "head max: {json}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_pushes_new_records_after_an_append() -> Result<(), String> {
    let served = serve_start_with("sse_new_records", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 2).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let items = vec![item("dev-new", "message", &json!({"text": DOLPHIN_TEXT}))];
    let landed = append(&base, &token, "k-sse-push", &append_body(&items)).await?;
    assert_eq!(landed.status, 200);
    let json = stream.recv_json_of("new-records").await?;
    let seq = json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {json}"))?;
    assert_eq!(seq, 3, "pushed seq: {json}");
    assert_eq!(
        json.get("device_id").and_then(Value::as_str),
        Some("dev-new")
    );
    let raw = format!("{json}");
    assert!(!raw.contains("dolphin 42"), "leak: {raw}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_init_seq_replays_the_missed_records() -> Result<(), String> {
    let served = serve_start_with("sse_catch_up", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 4).await?;
    let mut stream = open_account(&base, &token, Some(2)).await?;
    let _head = stream.recv_block().await?;
    let first = stream.recv_block().await?;
    assert_eq!(
        event_name(&first).as_deref(),
        Some("new-records"),
        "catch: {first}"
    );
    let json = data_json(&first)?;
    let seq = json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or(format!("no seq in {json}"))?;
    assert_eq!(seq, 3, "replay starts at 3: {json}");
    stream.close();
    served.stop().await
}
