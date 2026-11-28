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
use serde_json as _;
use sqlx as _;
use tracing as _;

use std::time::Duration;

use anybot_server_test::table_counts;
use anybot_server_test::typing_support::{
    BOT, QUIET, SETTLE, send_typing, stage, text_of, typing_body, watch,
};

#[tokio::test]
async fn typing_is_forwarded_not_stored() -> Result<(), String> {
    let stage = stage("typing_forwarded").await?;
    let mut phone = watch(&stage, &stage.phone).await?;
    let before = table_counts(&stage.served).await?;
    for text in ["He", "Hello", "Hello wor"] {
        let sent = send_typing(&stage, &stage.computer, &typing_body(BOT, "t1", text)).await?;
        assert_eq!(sent.status, 204, "{}", sent.json);
        let hint = phone.recv_json_of("typing").await?;
        assert_eq!(text_of(&hint), Some(text), "{hint}");
        assert_eq!(hint.get("task_id").and_then(|v| v.as_str()), Some("t1"));
        assert_eq!(
            hint.get("stream").and_then(|v| v.as_str()),
            Some(format!("bot/{BOT}").as_str())
        );
        let _woken = stage.served.clock.advance(Duration::from_secs(1));
    }
    assert_eq!(table_counts(&stage.served).await?, before);
    phone.close();
    stage.served.stop().await
}

#[tokio::test]
async fn typing_coalesces_to_one_per_second() -> Result<(), String> {
    let stage = stage("typing_coalesce").await?;
    let mut phone = watch(&stage, &stage.phone).await?;
    for n in 0..10 {
        let text = format!("answer {n}");
        let sent = send_typing(&stage, &stage.computer, &typing_body(BOT, "t1", &text)).await?;
        assert_eq!(sent.status, 204, "{}", sent.json);
    }
    let first = phone.recv_json_of("typing").await?;
    assert_eq!(text_of(&first), Some("answer 0"), "{first}");
    test_harness::sleep_real(SETTLE).await;
    assert!(
        phone.recv_within(QUIET).await.is_err(),
        "second hint came early"
    );
    let _woken = stage.served.clock.advance(Duration::from_secs(1));
    let second = phone.recv_json_of("typing").await?;
    assert_eq!(text_of(&second), Some("answer 9"), "{second}");
    assert!(
        phone.recv_within(QUIET).await.is_err(),
        "more than one hint in the second"
    );
    phone.close();
    stage.served.stop().await
}

#[tokio::test]
async fn typing_connection_opened_late_gets_the_next_hint_at_once() -> Result<(), String> {
    let stage = stage("typing_late").await?;
    let early = send_typing(&stage, &stage.computer, &typing_body(BOT, "t1", "before")).await?;
    assert_eq!(early.status, 204, "{}", early.json);
    let mut phone = watch(&stage, &stage.phone).await?;
    let later = send_typing(
        &stage,
        &stage.computer,
        &typing_body(BOT, "t1", "before and after"),
    )
    .await?;
    assert_eq!(later.status, 204, "{}", later.json);
    let hint = phone.recv_json_of("typing").await?;
    assert_eq!(text_of(&hint), Some("before and after"), "{hint}");
    phone.close();
    stage.served.stop().await
}

#[tokio::test]
async fn typing_of_different_tasks_is_not_merged() -> Result<(), String> {
    let stage = stage("typing_tasks").await?;
    let mut phone = watch(&stage, &stage.phone).await?;
    send_typing(
        &stage,
        &stage.computer,
        &typing_body(BOT, "one", "first task"),
    )
    .await?;
    send_typing(
        &stage,
        &stage.computer,
        &typing_body(BOT, "two", "second task"),
    )
    .await?;
    let a = phone.recv_json_of("typing").await?;
    let b = phone.recv_json_of("typing").await?;
    assert_eq!(text_of(&a), Some("first task"), "{a}");
    assert_eq!(text_of(&b), Some("second task"), "{b}");
    phone.close();
    stage.served.stop().await
}
