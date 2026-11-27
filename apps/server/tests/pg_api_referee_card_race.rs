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
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_api_referee_shared;
use pg_api_referee_shared::{
    answered_by, card_answer, count_of, login_device, plain_action, referee, show_card, start,
    task_record,
};

const ANSWER_RECORDS: &str = "SELECT count(*) AS n FROM records WHERE kind = 'referee_card_answer'";
const ANSWER_ROWS: &str = "SELECT count(*) AS n FROM card_answers";

#[tokio::test]
async fn a_second_answer_names_the_device_that_answered_first() -> Result<(), String> {
    let served = serve_start_with("referee_second", &ReleaseTable::empty()).await?;
    let computer = start(&served).await?;
    let phone = login_device(&served, "k-referee-device-b").await?;
    show_card(&served, &computer.token, "k-shown-1", "card-1").await?;
    let first = referee(
        &served,
        &computer.token,
        "k-answer-1",
        &card_answer(Some("card-1"), "allow"),
    )
    .await?;
    assert_eq!(first.status, 200, "{}", first.json);
    let second = referee(
        &served,
        &phone.token,
        "k-answer-2",
        &card_answer(Some("card-1"), "deny"),
    )
    .await?;
    assert_eq!(second.status, 409, "{}", second.json);
    assert_eq!(second.code().as_deref(), Some("card-already-answered"));
    assert_eq!(
        second.json.pointer("/params/device"),
        Some(&json!(computer.id)),
        "{}",
        second.json
    );
    assert_eq!(answered_by(&served, "card-1").await?, computer.id);
    assert_eq!(count_of(&served, ANSWER_RECORDS).await?, 1);
    served.stop().await
}

#[tokio::test]
async fn two_answers_arriving_together_leave_exactly_one() -> Result<(), String> {
    let served = serve_start_with("referee_together", &ReleaseTable::empty()).await?;
    let computer = start(&served).await?;
    let phone = login_device(&served, "k-referee-device-b").await?;
    show_card(&served, &computer.token, "k-shown-1", "card-1").await?;
    let allow = card_answer(Some("card-1"), "allow");
    let deny = card_answer(Some("card-1"), "deny");
    let (from_computer, from_phone) = tokio::join!(
        referee(&served, &computer.token, "k-answer-1", &allow),
        referee(&served, &phone.token, "k-answer-2", &deny),
    );
    let mut statuses = [from_computer?.status, from_phone?.status];
    statuses.sort_unstable();
    assert_eq!(statuses, [200, 409], "exactly one answer counts");
    assert_eq!(count_of(&served, ANSWER_ROWS).await?, 1);
    assert_eq!(count_of(&served, ANSWER_RECORDS).await?, 1);
    served.stop().await
}

#[tokio::test]
async fn a_card_shown_after_a_stop_can_be_answered_again() -> Result<(), String> {
    let served = serve_start_with("referee_after_stop", &ReleaseTable::empty()).await?;
    let computer = start(&served).await?;
    show_card(&served, &computer.token, "k-shown-1", "card-1").await?;
    let stop = referee(&served, &computer.token, "k-stop-1", &plain_action("stop")).await?;
    assert_eq!(stop.status, 200, "{}", stop.json);
    task_record(
        &served,
        &computer.token,
        "k-shown-2",
        "card_shown",
        &json!({"action": "shell", "card": "card-2"}),
    )
    .await?;
    let answer = referee(
        &served,
        &computer.token,
        "k-answer-1",
        &card_answer(Some("card-2"), "allow"),
    )
    .await?;
    assert_eq!(answer.status, 200, "{}", answer.json);
    served.stop().await
}
