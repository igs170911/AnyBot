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

use anybot_app_test::clockwalk::ClockWalker;
use std::time::Duration;

use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::serve::serve_start_with;
use anybot_app_test::sse::{SseOpen, data_json, event_name};
use anybot_domain::version_release::ReleaseTable;

mod pg_api_streams_sse_shared;
use pg_api_streams_sse_shared::{
    DOLPHIN_TEXT, append, append_body, item, login_bearer, open_account, progress_seq, seed,
};

#[tokio::test]
async fn t_conn_heartbeat_1() -> Result<(), String> {
    let served = serve_start_with("sse_heartbeat", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let mut walker = ClockWalker::manual(&served.clock);
    let mut total = 0usize;
    for slot in 0..4usize {
        let _warm = stream.drain_now();
        let _quiet = walker.step(Duration::from_secs(14)).await?;
        let _beat = walker.step(Duration::from_secs(1)).await?;
        let got = stream
            .count_heartbeats_within(Duration::from_millis(200))
            .await?;
        let _tail = stream.drain_now();
        assert_eq!(got, 1, "heartbeat slot {slot}: {got}");
        total = total
            .checked_add(got)
            .ok_or("heartbeat total overflow".to_owned())?;
    }
    walker.stop().await?;
    assert_eq!(total, 4, "heartbeat total: {total}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn t_conn_sse_5() -> Result<(), String> {
    let served = serve_start_with("sse_s5", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 45).await?;
    let watermark = "dev-39";
    let mut stream = open_account(&base, &token, Some(40)).await?;
    let head = stream.recv_block().await?;
    assert_eq!(
        event_name(&head).as_deref(),
        Some("progress"),
        "head: {head}"
    );
    let head_json = data_json(&head)?;
    assert_eq!(progress_seq(&head_json)?, 45, "head max: {head_json}");
    let missed = fetch_after(
        &base,
        &token,
        &format!("/v1/streams/after?stream=account%2Falice&after=40&device_at={watermark}"),
    )
    .await?;
    assert_eq!(missed.status, 200, "after: {}", missed.json);
    let got = seqs_of(&missed)?;
    assert_eq!(got, vec![41, 42, 43, 44, 45], "after seqs: {got:?}");
    let blue = json!({"text": DOLPHIN_TEXT});
    let landed = append(
        &base,
        &token,
        "k-sse-s5-push",
        &append_body(&[item("dev-s5", "message", &blue)]),
    )
    .await?;
    assert_eq!(landed.status, 200, "push: {}", landed.json);
    let replayed = seqs_of_events(&mut stream, 5).await?;
    assert_eq!(replayed, vec![41, 42, 43, 44, 45], "replay: {replayed:?}");
    let pushed = stream.recv_json_of("new-records").await?;
    assert_eq!(
        pushed.get("seq").and_then(Value::as_i64),
        Some(46),
        "push: {pushed}"
    );
    let raw = format!("{head}{replayed:?}{pushed}");
    assert!(!raw.contains("dolphin 42"), "leak: {raw}");
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_closes_within_five_seconds_when_credentials_are_revoked() -> Result<(), String> {
    let served = serve_start_with("sse_revoke", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let _head = stream.recv_block().await?;
    let logout = post_ok(
        &base,
        &token,
        "k-sse-logout",
        "/v1/account/logout",
        r#"{"ok": true}"#,
    )
    .await?;
    assert_eq!(logout.status, 200, "logout: {}", logout.json);
    let mut walker = ClockWalker::start(&served.clock);
    let ended = stream.recv_ended(Duration::from_secs(5)).await;
    walker.stop().await?;
    ended.map_err(|e| format!("revoked sse must end: {e}"))?;
    stream.close();
    served.stop().await
}

#[tokio::test]
async fn sse_carries_the_accel_buffering_header() -> Result<(), String> {
    let served = serve_start_with("sse_accel", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 1).await?;
    let mut stream = open_account(&base, &token, None).await?;
    let values = stream
        .headers
        .get_all("x-accel-buffering")
        .iter()
        .map(|v| v.to_str().unwrap_or_default().to_owned())
        .collect::<Vec<String>>();
    assert_eq!(values, vec!["no".to_owned()], "accel header: {values:?}");
    stream.close();
    served.stop().await
}
async fn seqs_of_events(stream: &mut SseOpen, want: usize) -> Result<Vec<i64>, String> {
    let mut out: Vec<i64> = Vec::new();
    for _round in 0..want {
        let json = stream.recv_json_of("new-records").await?;
        let seq = json
            .get("seq")
            .and_then(Value::as_i64)
            .ok_or(format!("no seq in {json}"))?;
        out.push(seq);
    }
    Ok(out)
}

async fn fetch_after(base: &str, token: &str, path: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new("").with_bearer(token);
    get(base, path, &spec).await
}

fn seqs_of(answer: &ResponseSpec) -> Result<Vec<i64>, String> {
    let rows = answer
        .json
        .get("records")
        .and_then(Value::as_array)
        .ok_or(format!("no records in {}", answer.json))?;
    rows.iter()
        .map(|r| {
            r.get("seq")
                .and_then(Value::as_i64)
                .ok_or(format!("no seq in {r}"))
        })
        .collect::<Result<Vec<i64>, String>>()
}

async fn post_ok(
    base: &str,
    token: &str,
    key: &str,
    path: &str,
    body: &str,
) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, path, &spec).await
}
