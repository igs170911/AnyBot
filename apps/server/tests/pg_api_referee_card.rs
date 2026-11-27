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
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_api_referee_shared;
use pg_api_referee_shared::{
    answered_by, card_answer, count_of, plain_action, referee, show_card, start, task_record,
};

const RECORDS: &str = "SELECT count(*) AS n FROM records";
const ANSWER_RECORDS: &str = "SELECT count(*) AS n FROM records WHERE kind = 'referee_card_answer'";
const ANSWER_ROWS: &str = "SELECT count(*) AS n FROM card_answers";

#[tokio::test]
async fn t_phone_cards_3() -> Result<(), String> {
    let served = serve_start_with("referee_no_card_id", &ReleaseTable::empty()).await?;
    let phone = start(&served).await?;
    show_card(&served, &phone.token, "k-shown-1", "card-1").await?;
    let before = count_of(&served, RECORDS).await?;
    let stale = referee(
        &served,
        &phone.token,
        "k-stale-1",
        &card_answer(None, "allow"),
    )
    .await?;
    assert_eq!(stale.status, 409, "{}", stale.json);
    assert_eq!(stale.code().as_deref(), Some("card-answer-stale"));
    assert_eq!(
        count_of(&served, RECORDS).await?,
        before,
        "nothing recorded"
    );
    assert_eq!(
        count_of(&served, ANSWER_ROWS).await?,
        0,
        "no answer counted"
    );
    served.stop().await
}

#[tokio::test]
async fn a_card_the_task_waits_for_takes_its_answer_once() -> Result<(), String> {
    let served = serve_start_with("referee_waiting", &ReleaseTable::empty()).await?;
    let phone = start(&served).await?;
    show_card(&served, &phone.token, "k-shown-1", "card-1").await?;
    let first = referee(
        &served,
        &phone.token,
        "k-answer-1",
        &card_answer(Some("card-1"), "allow"),
    )
    .await?;
    assert_eq!(first.status, 200, "{}", first.json);
    assert_eq!(answered_by(&served, "card-1").await?, phone.id);
    assert_eq!(count_of(&served, ANSWER_RECORDS).await?, 1);
    served.stop().await
}

#[tokio::test]
async fn a_card_nobody_showed_is_no_longer_waiting() -> Result<(), String> {
    let served = serve_start_with("referee_unknown_card", &ReleaseTable::empty()).await?;
    let phone = start(&served).await?;
    let before = count_of(&served, RECORDS).await?;
    let refused = referee(
        &served,
        &phone.token,
        "k-answer-1",
        &card_answer(Some("card-ghost"), "allow"),
    )
    .await?;
    assert_eq!(refused.status, 409, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("card-not-waiting"));
    assert_eq!(count_of(&served, RECORDS).await?, before);
    assert_eq!(count_of(&served, ANSWER_ROWS).await?, 0);
    served.stop().await
}

#[tokio::test]
async fn a_card_replaced_by_the_next_one_is_no_longer_waiting() -> Result<(), String> {
    let served = serve_start_with("referee_replaced", &ReleaseTable::empty()).await?;
    let phone = start(&served).await?;
    show_card(&served, &phone.token, "k-shown-1", "card-1").await?;
    show_card(&served, &phone.token, "k-shown-2", "card-2").await?;
    let old = referee(
        &served,
        &phone.token,
        "k-answer-1",
        &card_answer(Some("card-1"), "deny"),
    )
    .await?;
    assert_eq!(old.status, 409, "{}", old.json);
    assert_eq!(old.code().as_deref(), Some("card-not-waiting"));
    let current = referee(
        &served,
        &phone.token,
        "k-answer-2",
        &card_answer(Some("card-2"), "allow"),
    )
    .await?;
    assert_eq!(current.status, 200, "{}", current.json);
    assert_eq!(count_of(&served, ANSWER_ROWS).await?, 1);
    served.stop().await
}

#[tokio::test]
async fn a_stopped_or_cancelled_task_has_no_card_waiting() -> Result<(), String> {
    let served = serve_start_with("referee_stopped", &ReleaseTable::empty()).await?;
    let phone = start(&served).await?;
    show_card(&served, &phone.token, "k-shown-1", "card-1").await?;
    let stop = referee(&served, &phone.token, "k-stop-1", &plain_action("stop")).await?;
    assert_eq!(stop.status, 200, "{}", stop.json);
    let late = referee(
        &served,
        &phone.token,
        "k-answer-1",
        &card_answer(Some("card-1"), "allow"),
    )
    .await?;
    assert_eq!(late.status, 409, "{}", late.json);
    assert_eq!(late.code().as_deref(), Some("card-not-waiting"));
    task_record(
        &served,
        &phone.token,
        "k-shown-2",
        "card_shown",
        &serde_json::json!({"action": "shell", "card": "card-2"}),
    )
    .await?;
    let cancel = referee(&served, &phone.token, "k-cancel-1", &plain_action("cancel")).await?;
    assert_eq!(cancel.status, 200, "{}", cancel.json);
    let after_cancel = referee(
        &served,
        &phone.token,
        "k-answer-2",
        &card_answer(Some("card-2"), "allow"),
    )
    .await?;
    assert_eq!(after_cancel.code().as_deref(), Some("card-not-waiting"));
    assert_eq!(count_of(&served, ANSWER_ROWS).await?, 0);
    served.stop().await
}
