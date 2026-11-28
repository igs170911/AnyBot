use serde_json::json;

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

use anybot_app_test::http::{RequestSpec, get};
use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_task_status_shared;

use pg_task_status_shared::{
    ACCOUNT, answer, append, append_body, bot_create, bot_uuid, dev_uuid, item, lifecycle,
    login_bearer, login_bob, records_count, result_code, result_seq, result_tag, status,
    status_expect, status_with_key, task_uuid, thumbs, thumbs_at, value_at,
};

#[tokio::test]
async fn status_of_unknown_task_is_answered_missing() -> Result<(), String> {
    let served = serve_start_with("status_missing", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let created = append(
        &base,
        &token,
        "k-miss-1",
        &append_body(ACCOUNT, &bot_create("bot-4")),
    )
    .await?;
    assert_eq!(result_seq(&created, 0)?, 1, "create: {}", created.json);
    let answer = status_expect(&base, &token, &task_uuid(), "bot-4", 404).await?;
    assert_eq!(answer.status, 404, "missing task: {}", answer.json);
    assert_eq!(
        answer.code().unwrap_or_default(),
        "task-not-found",
        "missing code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn status_with_unknown_bot_is_answered_not_found() -> Result<(), String> {
    let served = serve_start_with("status_bot", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let answer = status_expect(&base, &token, "task-1", &bot_uuid(), 403).await?;
    assert_eq!(answer.status, 403, "missing bot: {}", answer.json);
    assert_eq!(
        answer.code().unwrap_or_default(),
        "stream-bot-missing",
        "bot code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn status_of_other_account_bot_is_forbidden() -> Result<(), String> {
    let served = serve_start_with("status_foreign", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-forg-1",
        &append_body(ACCOUNT, &bot_create("bot-5")),
    )
    .await?;
    let bob = login_bob(&served).await?;
    let foreign = append(
        &base,
        &bob,
        "k-forg-2",
        &append_body(
            "bot-5",
            &thumbs_at("task-1", 1, "dev-f", "thumbs_up", 1_700_000_000_600),
        ),
    )
    .await?;
    assert_eq!(foreign.status, 200, "foreign append: {}", foreign.json);
    assert_eq!(
        result_code(&foreign, 0)?,
        "record-no-permission",
        "foreign append code: {}",
        foreign.json
    );
    assert_eq!(
        result_tag(&foreign, 0)?,
        "rejected",
        "foreign: {}",
        foreign.json
    );
    let answer = status_expect(&base, &bob, "task-1", "bot-5", 403).await?;
    assert_eq!(answer.status, 403, "foreign bot: {}", answer.json);
    assert_eq!(
        answer.code().unwrap_or_default(),
        "record-no-permission",
        "foreign code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn status_needs_no_idempotency_key_and_writes_nothing() -> Result<(), String> {
    let served = serve_start_with("status_read", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-read-1",
        &append_body(ACCOUNT, &bot_create("bot-6")),
    )
    .await?;
    let device = dev_uuid();
    let task = "task-6";
    let opened = append(
        &base,
        &token,
        "k-read-2",
        &append_body("bot-6", &answer(&device, task, "hello")),
    )
    .await?;
    assert_eq!(
        result_tag(&opened, 0)?,
        "accepted",
        "opened: {}",
        opened.json
    );
    assert_eq!(result_seq(&opened, 1)?, 2, "opened seq: {}", opened.json);
    let keyed = status_with_key(&base, &token, task, "bot-6", "k-read-key").await?;
    assert_eq!(keyed.status, 200, "key ignored: {}", keyed.json);
    let plain = status(&base, &token, task, "bot-6").await?;
    assert_eq!(
        plain.json, keyed.json,
        "same answer: {} vs {}",
        plain.json, keyed.json
    );
    assert_eq!(
        value_at(&plain.json, &["status"]),
        Some(json!("running")),
        "folded: {}",
        plain.json
    );
    let count = records_count(&base, &token, "bot-6").await?;
    assert_eq!(count, 2, "no writes from status: {count}");
    served.stop().await
}

#[tokio::test]
async fn task_status_reads_deleted_bot() -> Result<(), String> {
    let served = serve_start_with("status_deleted", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-dl-1",
        &append_body(ACCOUNT, &bot_create("bot-11")),
    )
    .await?;
    let device = dev_uuid();
    let task = "task-11";
    let landed = append(
        &base,
        &token,
        "k-dl-2",
        &append_body("bot-11", &lifecycle(task, "bot-11", "the report is ready")),
    )
    .await?;
    let answer_seq = result_seq(&landed, 1)?;
    let _up = append(
        &base,
        &token,
        "k-dl-3",
        &append_body("bot-11", &thumbs(task, answer_seq, &device, "thumbs_up")),
    )
    .await?;
    let delete = vec![item("dev-x", "bots_delete", &json!({ "bot_id": "bot-11" }))];
    let gone = append(&base, &token, "k-dl-4", &append_body(ACCOUNT, &delete)).await?;
    assert_eq!(result_tag(&gone, 0)?, "accepted", "delete: {}", gone.json);
    let answer = status(&base, &token, task, "bot-11").await?;
    assert_eq!(answer.status, 200, "deleted bot: {}", answer.json);
    assert_eq!(
        value_at(&answer.json, &["status"]),
        Some(json!("completed")),
        "folded state: {}",
        answer.json
    );
    assert_eq!(
        value_at(&answer.json, &["report", "userConfirmed"]),
        Some(json!(true)),
        "keeps confirmed: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn status_needs_a_bearer_token() -> Result<(), String> {
    let served = serve_start_with("status_auth", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let spec = RequestSpec::new("");
    let answer = get(&base, "/v1/tasks/task-1/status?bot=bot-1", &spec).await?;
    assert_eq!(answer.status, 401, "no token: {}", answer.json);
    served.stop().await
}

#[tokio::test]
async fn status_without_a_bot_query_is_refused() -> Result<(), String> {
    let served = serve_start_with("status_no_bot", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let spec = RequestSpec::new("").with_bearer(&token);
    let answer = get(&base, "/v1/tasks/task-1/status", &spec).await?;
    assert_eq!(answer.status, 400, "missing bot: {}", answer.json);
    served.stop().await
}
