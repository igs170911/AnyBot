use anybot_app_server as _;
use anybot_app_test as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use tracing as _;

use anybot_server_test::typing_support::{
    BOT, QUIET, SETTLE, send_typing, stage, text_of, typing_body, watch,
};

#[tokio::test]
async fn typing_only_reaches_owner() -> Result<(), String> {
    let stage = stage("typing_owner").await?;
    let mut owner = watch(&stage, &stage.phone).await?;
    let mut other = watch(&stage, &stage.bob).await?;
    let sent = send_typing(&stage, &stage.computer, &typing_body(BOT, "t1", "private")).await?;
    assert_eq!(sent.status, 204, "{}", sent.json);
    let hint = owner.recv_json_of("typing").await?;
    assert_eq!(text_of(&hint), Some("private"), "{hint}");
    test_harness::sleep_real(SETTLE).await;
    assert!(
        other.recv_within(QUIET).await.is_err(),
        "bob saw a typing hint"
    );
    owner.close();
    other.close();
    stage.served.stop().await
}

#[tokio::test]
async fn typing_rejects_browser_credential() -> Result<(), String> {
    let stage = stage("typing_browser").await?;
    let mut phone = watch(&stage, &stage.phone).await?;
    let sent = send_typing(&stage, &stage.phone, &typing_body(BOT, "t1", "hello")).await?;
    assert_eq!(sent.status, 403, "{}", sent.json);
    assert_eq!(sent.code().as_deref(), Some("device-no-permission"));
    test_harness::sleep_real(SETTLE).await;
    assert!(
        phone.recv_within(QUIET).await.is_err(),
        "a refused hint was forwarded"
    );
    phone.close();
    stage.served.stop().await
}

#[tokio::test]
async fn typing_for_unknown_bot_is_refused() -> Result<(), String> {
    let stage = stage("typing_unknown_bot").await?;
    let sent = send_typing(
        &stage,
        &stage.computer,
        &typing_body("no-such-bot", "t1", "x"),
    )
    .await?;
    assert_eq!(sent.status, 403, "{}", sent.json);
    assert_eq!(sent.code().as_deref(), Some("stream-bot-missing"));
    stage.served.stop().await
}

#[tokio::test]
async fn typing_for_account_stream_is_refused() -> Result<(), String> {
    let stage = stage("typing_account_stream").await?;
    let body = serde_json::json!({
        "stream": "account",
        "task_id": "t1",
        "text": "x",
        "thinking": "",
    })
    .to_string();
    let sent = send_typing(&stage, &stage.computer, &body).await?;
    assert_eq!(sent.status, 400, "{}", sent.json);
    stage.served.stop().await
}

#[tokio::test]
async fn typing_body_over_the_write_limit_is_413() -> Result<(), String> {
    let stage = stage("typing_too_large").await?;
    let huge = "a".repeat(300 * 1024);
    let sent = send_typing(&stage, &stage.computer, &typing_body(BOT, "t1", &huge)).await?;
    assert_eq!(sent.status, 413, "{}", sent.json);
    stage.served.stop().await
}
