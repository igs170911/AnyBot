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

use anybot_app_test::http::ResponseSpec;
use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;
mod pg_task_status_shared;

use pg_task_status_shared::{
    ACCOUNT, answer, append, append_body, bot_create, bot_uuid, dev_uuid, lifecycle, login_bearer,
    login_bob, records_count, result_code, result_seq, result_tag, status_expect, status_with_key,
    task_event_item, task_uuid, thumbs, thumbs_at, value_at,
};

fn confirmed_read(data: &Value, at_ms: i64) {
    assert_eq!(
        value_at(data, &["status"]),
        Some(json!("completed")),
        "status: {data}"
    );
    assert_eq!(
        value_at(data, &["detail", "detail"]),
        Some(json!("user_confirmed")),
        "detail: {data}"
    );
    assert_eq!(
        value_at(data, &["report", "userConfirmed"]),
        Some(json!(true)),
        "report: {data}"
    );
    assert_eq!(
        value_at(data, &["report", "userConfirmedAtMs"]),
        Some(json!(at_ms)),
        "confirm time: {data}"
    );
}

#[tokio::test]
async fn t_cards_n1_1() -> Result<(), String> {
    let served = serve_start_with("cards_n1_1", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let bot = bot_uuid();
    let _created = append(
        &base,
        &token,
        "k-n11-1",
        &append_body(ACCOUNT, &bot_create(&bot)),
    )
    .await?;
    let task = task_uuid();
    let landed = append(
        &base,
        &token,
        "k-n11-2",
        &append_body(&bot, &lifecycle(&task, &bot, "the report is ready")),
    )
    .await?;
    let answer_seq = result_seq(&landed, 1)?;
    let up = append(
        &base,
        &token,
        "k-n11-3",
        &append_body(
            &bot,
            &thumbs_at(
                &task,
                answer_seq,
                &dev_uuid(),
                "thumbs_up",
                1_700_000_000_500,
            ),
        ),
    )
    .await?;
    assert_eq!(result_tag(&up, 0)?, "accepted", "thumbs: {}", up.json);
    let got = status_expect(&base, &token, &task, &bot, 200).await?;
    confirmed_read(&got.json, 1_700_000_000_500_i64);
    let keyed = status_with_key(&base, &token, &task, &bot, "k-n11-get").await?;
    assert_eq!(keyed.json, got.json, "keyed read: {}", keyed.json);
    let count = records_count(&base, &token, &bot).await?;
    assert_eq!(count, 5, "task rows: {count}");
    let bob = login_bob(&served).await?;
    let kept = status_expect(&base, &bob, &task, &bot, 403).await?;
    assert_eq!(kept.status, 403, "bob read: {}", kept.json);
    served.stop().await
}

#[tokio::test]
async fn t_cards_n1_2() -> Result<(), String> {
    let served = serve_start_with("cards_n1_2", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let bot = bot_uuid();
    let _created = append(
        &base,
        &token,
        "k-n12-1",
        &append_body(ACCOUNT, &bot_create(&bot)),
    )
    .await?;
    let task = task_uuid();
    let first = append(
        &base,
        &token,
        "k-n12-2",
        &append_body(&bot, &answer(&dev_uuid(), &task, "write the report")),
    )
    .await?;
    let answer_seq = result_seq(&first, 1)?;
    let up = append(
        &base,
        &token,
        "k-n12-3",
        &append_body(&bot, &thumbs(&task, answer_seq, &dev_uuid(), "thumbs_up")),
    )
    .await?;
    assert_eq!(result_tag(&up, 0)?, "accepted", "thumbs: {}", up.json);
    let stray = append(
        &base,
        &token,
        "k-n12-5",
        &append_body(&bot, &thumbs(&task, 99, &dev_uuid(), "thumbs_up")),
    )
    .await?;
    assert_eq!(result_tag(&stray, 0)?, "rejected", "stray: {}", stray.json);
    assert_eq!(
        result_code(&stray, 0)?,
        "thumbs-target-invalid",
        "stray code: {}",
        stray.json
    );
    running_read(&status_expect(&base, &token, &task, &bot, 200).await?);
    finish_and_check(&base, &token, &bot, &task).await?;
    served.stop().await
}

fn running_read(running: &ResponseSpec) {
    let data = &running.json;
    assert_eq!(
        value_at(data, &["status"]),
        Some(json!("running")),
        "running: {}",
        running.json
    );
    assert_eq!(
        value_at(data, &["report", "userConfirmed"]),
        Some(json!(false)),
        "running: {}",
        running.json
    );
}

async fn finish_and_check(base: &str, token: &str, bot: &str, task: &str) -> Result<(), String> {
    let ended = json!({ "reason": { "kind": "answered" }, "reply_delivered": true });
    let report = json!({ "kind": "shown", "file": "report.txt" });
    let _finished = append(
        base,
        token,
        "k-n12-4",
        &append_body(
            bot,
            &[
                task_event_item(&dev_uuid(), "turn_ended", task, &ended, 1_700_000_000_300),
                task_event_item(
                    &dev_uuid(),
                    "device_report",
                    task,
                    &report,
                    1_700_000_000_400,
                ),
            ],
        ),
    )
    .await?;
    let done = status_expect(base, token, task, bot, 200).await?;
    let data = &done.json;
    assert_eq!(
        value_at(data, &["status"]),
        Some(json!("completed")),
        "done: {}",
        done.json
    );
    assert_eq!(
        value_at(data, &["detail", "detail"]),
        Some(json!("delivered")),
        "done: {}",
        done.json
    );
    assert_eq!(
        value_at(data, &["report", "userConfirmed"]),
        Some(json!(false)),
        "done: {}",
        done.json
    );
    Ok(())
}

#[tokio::test]
async fn t_cards_n1_3() -> Result<(), String> {
    let served = serve_start_with("cards_n1_3", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let bot = bot_uuid();
    let _created = append(
        &base,
        &token,
        "k-n13-1",
        &append_body(ACCOUNT, &bot_create(&bot)),
    )
    .await?;
    let task = task_uuid();
    let landed = append(
        &base,
        &token,
        "k-n13-2",
        &append_body(&bot, &lifecycle(&task, &bot, "the report is ready")),
    )
    .await?;
    let answer_seq = result_seq(&landed, 1)?;
    let _up = append(
        &base,
        &token,
        "k-n13-3",
        &append_body(&bot, &thumbs(&task, answer_seq, &dev_uuid(), "thumbs_up")),
    )
    .await?;
    let removed = append(
        &base,
        &token,
        "k-n13-4",
        &append_body(
            &bot,
            &thumbs(&task, answer_seq, &dev_uuid(), "thumbs_up_removed"),
        ),
    )
    .await?;
    assert_eq!(
        result_tag(&removed, 0)?,
        "accepted",
        "removed: {}",
        removed.json
    );
    let got = status_expect(&base, &token, &task, &bot, 200).await?;
    confirmed_read(&got.json, 1_700_000_000_500_i64);
    served.stop().await
}
