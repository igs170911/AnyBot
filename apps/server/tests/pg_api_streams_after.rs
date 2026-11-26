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
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

fn item(device: &str, text: &str) -> Value {
    json!({
        "device_id": device,
        "kind": "message",
        "content": {"text": text},
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    })
}

fn append_body(items: &[Value]) -> String {
    json!({"stream": "account", "items": items}).to_string()
}

async fn login_bearer(served: &anybot_app_test::serve::Served) -> Result<String, String> {
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(served).await?;
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-after-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

async fn append(base: &str, token: &str, key: &str, body: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new(body).with_key(key).with_bearer(token);
    post(base, "/v1/streams/append", &spec).await
}

async fn seed(base: &str, token: &str, count: usize) -> Result<(), String> {
    for index in 0..count {
        let items = vec![item(&format!("dev-{index}"), &format!("text-{index}"))];
        let answer = append(
            base,
            token,
            &format!("k-seed-{index}"),
            &append_body(&items),
        )
        .await?;
        assert_eq!(answer.status, 200, "seed {index}: {}", answer.json);
    }
    Ok(())
}

fn after_path(after: i64, device_at: Option<&str>) -> String {
    match device_at {
        Some(at) => {
            format!("/v1/streams/after?stream=account%2Falice&after={after}&device_at={at}")
        }
        None => format!("/v1/streams/after?stream=account%2Falice&after={after}"),
    }
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
        .collect()
}

fn flag_of(answer: &ResponseSpec, key: &str) -> Result<bool, String> {
    answer
        .json
        .get(key)
        .and_then(Value::as_bool)
        .ok_or(format!("no {key} in {}", answer.json))
}

#[tokio::test]
async fn after_two_returns_the_next_records_in_order() -> Result<(), String> {
    let served = serve_start_with("after_order", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 4).await?;
    let answer = fetch_after(&base, &token, &after_path(2, Some("dev-1"))).await?;
    assert_eq!(answer.status, 200, "after: {}", answer.json);
    assert_eq!(seqs_of(&answer)?, vec![3, 4], "page: {}", answer.json);
    assert!(!flag_of(&answer, "has_more")?, "has_more: {}", answer.json);
    served.stop().await
}

#[tokio::test]
async fn after_zero_replays_the_whole_stream() -> Result<(), String> {
    let served = serve_start_with("after_zero", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 3).await?;
    let answer = fetch_after(&base, &token, &after_path(0, None)).await?;
    assert_eq!(answer.status, 200, "after 0: {}", answer.json);
    assert_eq!(seqs_of(&answer)?, vec![1, 2, 3], "whole: {}", answer.json);
    served.stop().await
}

#[tokio::test]
async fn after_with_the_wrong_record_device_says_ahead() -> Result<(), String> {
    let served = serve_start_with("after_mismatch", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 3).await?;
    let answer = fetch_after(&base, &token, &after_path(2, Some("dev-9"))).await?;
    assert_eq!(answer.status, 409, "mismatch: {}", answer.json);
    assert_eq!(answer.code().unwrap_or_default(), "sync-after-ahead");
    served.stop().await
}

#[tokio::test]
async fn after_past_the_server_says_ahead_with_the_max() -> Result<(), String> {
    let served = serve_start_with("after_ahead", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 3).await?;
    let answer = fetch_after(&base, &token, &after_path(9, None)).await?;
    assert_eq!(answer.status, 409, "ahead: {}", answer.json);
    assert_eq!(answer.code().unwrap_or_default(), "sync-after-ahead");
    let title = answer
        .json
        .get("title")
        .and_then(Value::as_str)
        .ok_or(format!("no title in {}", answer.json))?;
    assert!(
        title.contains("max_seq 3"),
        "title carries the max: {title}"
    );
    served.stop().await
}

#[tokio::test]
async fn after_capped_page_carries_the_more_flag() -> Result<(), String> {
    let served = serve_start_with("after_page", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    for wave in 0..3 {
        let items: Vec<Value> = (0..500)
            .map(|n| item(&format!("dev-{wave}-{n}"), &format!("text-{wave}-{n}")))
            .collect();
        let answer = append(
            &base,
            &token,
            &format!("k-page-{wave}"),
            &append_body(&items),
        )
        .await?;
        assert_eq!(answer.status, 200, "wave {wave}: {}", answer.json);
    }
    let page = fetch_after(&base, &token, &after_path(0, None)).await?;
    assert_eq!(page.status, 200, "page: {}", page.json);
    assert_eq!(seqs_of(&page)?.len(), 1000, "page cap: {}", page.json);
    assert!(flag_of(&page, "has_more")?, "more flag: {}", page.json);
    served.stop().await
}

#[tokio::test]
async fn after_of_a_missing_stream_says_ahead() -> Result<(), String> {
    let served = serve_start_with("after_missing", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let answer = fetch_after(&base, &token, "/v1/streams/after?stream=bot%2Fnope&after=5").await?;
    assert_eq!(answer.status, 403, "missing bot: {}", answer.json);
    served.stop().await
}

#[tokio::test]
async fn after_of_other_account_stream_is_refused() -> Result<(), String> {
    let served = serve_start_with("after_foreign", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    seed(&base, &token, 2).await?;
    let bob_body = json!({
        "account": "bob",
        "password": "bobpassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&bob_body).with_key("k-bob-after");
    let mut walker = ClockWalker::start(&served.clock);
    let landed = post(&base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    let bob = landed?.bearer()?;
    let answer = fetch_after(&base, &bob, &after_path(1, Some("dev-0"))).await?;
    assert_eq!(answer.status, 403, "foreign after: {}", answer.json);
    served.stop().await
}
