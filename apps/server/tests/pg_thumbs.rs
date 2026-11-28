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

use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_task_status_shared;

use pg_task_status_shared::{
    ACCOUNT, answer, append, append_body, bot_create, bot_uuid, dev_uuid, item, lifecycle,
    login_bearer, login_bob, records_count, result_code, result_seq, result_tag, status,
    status_expect, status_with_key, task_uuid, thumbs, thumbs_at, value_at,
};

#[tokio::test]
async fn thumbs_up_rejects_non_answer_target() -> Result<(), String> {
    let served = serve_start_with("thumbs_no_target", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-th-1",
        &append_body(ACCOUNT, &bot_create("bot-7")),
    )
    .await?;
    let _opened = append(
        &base,
        &token,
        "k-th-2",
        &append_body("bot-7", &answer("dev-a", "task-7", "hello")),
    )
    .await?;
    let rejected = append(
        &base,
        &token,
        "k-th-3",
        &append_body("bot-7", &thumbs("task-7", 99, "dev-g", "thumbs_up")),
    )
    .await?;
    assert_eq!(
        result_tag(&rejected, 0)?,
        "rejected",
        "missing target: {}",
        rejected.json
    );
    assert_eq!(
        result_code(&rejected, 0)?,
        "thumbs-target-invalid",
        "target code: {}",
        rejected.json
    );
    served.stop().await
}

#[tokio::test]
async fn thumbs_up_rejects_message_target() -> Result<(), String> {
    let served = serve_start_with("thumbs_message", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-tm-1",
        &append_body(ACCOUNT, &bot_create("bot-8")),
    )
    .await?;
    let plain = vec![item("dev-z", "message", &json!({ "text": "chat only" }))];
    let landed = append(&base, &token, "k-tm-2", &append_body("bot-8", &plain)).await?;
    let seq = result_seq(&landed, 0)?;
    let rejected = append(
        &base,
        &token,
        "k-tm-3",
        &append_body(
            "bot-8",
            &thumbs_at("task-10", seq, "dev-g", "thumbs_up", 500),
        ),
    )
    .await?;
    assert_eq!(
        result_tag(&rejected, 0)?,
        "rejected",
        "message target: {}",
        rejected.json
    );
    assert_eq!(
        result_code(&rejected, 0)?,
        "thumbs-target-invalid",
        "message code: {}",
        rejected.json
    );
    served.stop().await
}

#[tokio::test]
async fn thumbs_up_rejects_other_task_target() -> Result<(), String> {
    let served = serve_start_with("thumbs_other_task", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-ot-1",
        &append_body(ACCOUNT, &bot_create("bot-9")),
    )
    .await?;
    let landed = append(
        &base,
        &token,
        "k-ot-2",
        &append_body("bot-9", &answer("dev-a", "task-9", "hello")),
    )
    .await?;
    let seq = result_seq(&landed, 0)?;
    let rejected = append(
        &base,
        &token,
        "k-ot-3",
        &append_body("bot-9", &thumbs("task-10", seq, "dev-g", "thumbs_up")),
    )
    .await?;
    assert_eq!(
        result_tag(&rejected, 0)?,
        "rejected",
        "other task target: {}",
        rejected.json
    );
    assert_eq!(
        result_code(&rejected, 0)?,
        "thumbs-target-invalid",
        "other task code: {}",
        rejected.json
    );
    served.stop().await
}

#[tokio::test]
async fn thumbs_from_another_account_is_rejected() -> Result<(), String> {
    let served = serve_start_with("thumbs_foreign", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _bot = append(
        &base,
        &token,
        "k-tf-1",
        &append_body(ACCOUNT, &bot_create("bot-14")),
    )
    .await?;
    let _opened = append(
        &base,
        &token,
        "k-tf-2",
        &append_body("bot-14", &answer("dev-a", "task-14", "hello")),
    )
    .await?;
    let bob = login_bob(&served).await?;
    let rejected = append(
        &base,
        &bob,
        "k-tf-3",
        &append_body("bot-14", &thumbs("task-14", 1, "dev-g", "thumbs_up")),
    )
    .await?;
    assert_eq!(
        result_tag(&rejected, 0)?,
        "rejected",
        "foreign thumbs: {}",
        rejected.json
    );
    assert_eq!(
        result_code(&rejected, 0)?,
        "record-no-permission",
        "foreign code: {}",
        rejected.json
    );
    served.stop().await
}

#[tokio::test]
async fn thumbs_up_lands_and_points_at_the_matching_answer() -> Result<(), String> {
    let served = serve_start_with("thumbs_ok", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let bot = bot_uuid();
    let _bot = append(
        &base,
        &token,
        "k-tk-1",
        &append_body(ACCOUNT, &bot_create(&bot)),
    )
    .await?;
    let device = dev_uuid();
    let task = task_uuid();
    let landed = append(
        &base,
        &token,
        "k-tk-2",
        &append_body(&bot, &lifecycle(&task, &bot, "the report is ready")),
    )
    .await?;
    let seq = result_seq(&landed, 1)?;
    let before = status_expect(&base, &token, &task, &bot, 200).await?;
    assert_eq!(before.status, 200, "before thumbs: {}", before.json);
    assert_eq!(
        value_at(&before.json, &["status"]),
        Some(json!("completed")),
        "before: {}",
        before.json
    );
    let up = append(
        &base,
        &token,
        "k-tk-3",
        &append_body(&bot, &thumbs(&task, seq, &device, "thumbs_up")),
    )
    .await?;
    assert_eq!(result_tag(&up, 0)?, "accepted", "valid thumbs: {}", up.json);
    assert_eq!(result_seq(&up, 0)?, 5, "thumbs seq: {}", up.json);
    lands_read(&base, &token, &task, &bot).await?;
    served.stop().await
}

async fn lands_read(base: &str, token: &str, task: &str, bot: &str) -> Result<(), String> {
    let keyed = status_with_key(base, token, task, bot, "k-tk-get").await?;
    assert_eq!(keyed.status, 200, "keyed: {}", keyed.json);
    let answer = status(base, token, task, bot).await?;
    assert_eq!(
        answer.json, keyed.json,
        "same answer: {} vs {}",
        answer.json, keyed.json
    );
    assert_eq!(
        value_at(&answer.json, &["report", "userConfirmed"]),
        Some(json!(true)),
        "confirmed after done: {}",
        answer.json
    );
    assert_eq!(
        value_at(&answer.json, &["status"]),
        Some(json!("completed")),
        "status: {}",
        answer.json
    );
    let count = records_count(base, token, bot).await?;
    assert_eq!(count, 5, "one thumbs row: {count}");
    Ok(())
}
