use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

mod pg_mailbox_shared;

use std::time::Duration;

use anybot_app_test::http::{RequestSpec, get};
use anybot_app_test::mailbox_prop::Seats;
use anybot_app_test::serve::{Served, serve_start};
use anybot_server_test::computers_support::{api_post, computer_login};
use anybot_server_test::mailbox_support::{
    claim, deliver, letter_state, pending_count, reassign_lines,
};
use pg_mailbox_shared::three_stand_ins;

const BOT: &str = "bot-reassign";

async fn touch(served: &Served, bearer: &str) -> Result<(), String> {
    let spec = RequestSpec::new("").with_bearer(bearer);
    let answer = get(&served.base_url(), "/v1/computers", &spec).await?;
    assert_eq!(answer.status, 200, "{}", answer.json);
    Ok(())
}

async fn start() -> Result<(Served, Seats), String> {
    let served = serve_start("mailbox_reassign").await?;
    let seats = three_stand_ins(&served, BOT).await?;
    touch(&served, &seats.a.bearer).await?;
    touch(&served, &seats.b.bearer).await?;
    Ok((served, seats))
}

async fn handed_to_a(served: &Served, seats: &Seats, key: &str) -> Result<String, String> {
    let handed = deliver(served, &seats.phone, BOT, Some(&seats.a.device_id), key).await?;
    assert_eq!(handed.status, 200, "{}", handed.json);
    assert_eq!(handed.field_str("device_id")?, seats.a.device_id);
    assert!(handed.json.get("reassigned").is_none());
    handed.field_str("letter_id")
}

#[tokio::test]
async fn a_claim_inside_ten_seconds_wins_and_one_after_it_is_refused() -> Result<(), String> {
    let (served, seats) = start().await?;
    let early = handed_to_a(&served, &seats, "k-early").await?;
    let _woken = served.clock.advance(Duration::from_secs(9));
    let won = claim(&served, &seats.a.bearer, &early, "k-claim-early").await?;
    assert_eq!(won.status, 200, "{}", won.json);
    assert_eq!(won.field_str("kind")?, "round");
    let late = handed_to_a(&served, &seats, "k-late").await?;
    let _woken = served.clock.advance(Duration::from_secs(10));
    let refused = claim(&served, &seats.a.bearer, &late, "k-claim-late").await?;
    assert_eq!(refused.status, 409, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("mailbox-claim-reassigned"));
    assert_eq!(letter_state(&served, &late).await?.state, "reassigned");
    assert_eq!(pending_count(&served, &seats.x.device_id).await?, 1);
    let lines = reassign_lines(&served, BOT).await?;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines
            .first()
            .is_some_and(|line| line.contains("mailbox-reassign-offline")),
        "{lines:?}"
    );
    served.stop().await
}

#[tokio::test]
async fn an_offline_computer_is_skipped_at_delivery_with_a_line_on_the_bot_stream()
-> Result<(), String> {
    let (served, seats) = start().await?;
    let _woken = served.clock.advance(Duration::from_secs(61));
    let handed = deliver(
        &served,
        &seats.phone,
        BOT,
        Some(&seats.a.device_id),
        "k-off",
    )
    .await?;
    assert_eq!(handed.status, 200, "{}", handed.json);
    assert_eq!(handed.field_str("device_id")?, seats.x.device_id);
    assert_eq!(handed.field_str("reassigned")?, "offline");
    let lines = reassign_lines(&served, BOT).await?;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines
            .first()
            .is_some_and(|line| line.contains(&seats.a.device_id)),
        "{lines:?}"
    );
    served.stop().await
}

#[tokio::test]
async fn a_computer_with_the_delegate_switch_off_is_replaced_with_that_reason() -> Result<(), String>
{
    let (served, seats) = start().await?;
    let quiet = computer_login(&served, "C", "k-c").await?;
    let handed = deliver(&served, &seats.phone, BOT, Some(&quiet.device_id), "k-del").await?;
    assert_eq!(handed.status, 200, "{}", handed.json);
    assert_eq!(handed.field_str("device_id")?, seats.x.device_id);
    assert_eq!(handed.field_str("reassigned")?, "delegate-off");
    let lines = reassign_lines(&served, BOT).await?;
    assert!(
        lines
            .first()
            .is_some_and(|line| line.contains("mailbox-reassign-delegate-off")),
        "{lines:?}"
    );
    served.stop().await
}

#[tokio::test]
async fn removing_a_computer_hands_its_waiting_round_to_the_bot_computer() -> Result<(), String> {
    let (served, seats) = start().await?;
    let waiting = handed_to_a(&served, &seats, "k-wait").await?;
    let body = serde_json::json!({"device_id": seats.a.device_id});
    let removed = api_post(
        &served,
        &seats.b.bearer,
        "/v1/computers/remove",
        &body,
        "k-rm",
    )
    .await?;
    assert_eq!(removed.status, 200, "{}", removed.json);
    assert_eq!(letter_state(&served, &waiting).await?.state, "reassigned");
    assert_eq!(pending_count(&served, &seats.x.device_id).await?, 1);
    served.stop().await
}

#[tokio::test]
async fn two_computers_claiming_one_letter_leave_it_to_the_addressee() -> Result<(), String> {
    let (served, seats) = start().await?;
    let letter = handed_to_a(&served, &seats, "k-two").await?;
    let (from_a, from_b) = tokio::join!(
        claim(&served, &seats.a.bearer, &letter, "k-two-a"),
        claim(&served, &seats.b.bearer, &letter, "k-two-b")
    );
    let (from_a, from_b) = (from_a?, from_b?);
    assert_eq!(from_a.status, 200, "{}", from_a.json);
    let code = from_b.code();
    assert!(
        matches!(
            (from_b.status, code.as_deref()),
            (404, Some("mailbox-letter-unknown")) | (409, Some("mailbox-claim-taken"))
        ),
        "{}",
        from_b.json
    );
    let after = letter_state(&served, &letter).await?;
    assert_eq!(
        after.claimed_by.as_deref(),
        Some(seats.a.device_id.as_str())
    );
    served.stop().await
}

#[tokio::test]
async fn waiting_letters_survive_a_restart_and_still_expire() -> Result<(), String> {
    let (mut served, seats) = start().await?;
    let waiting = handed_to_a(&served, &seats, "k-restart").await?;
    served.stop_server().await?;
    served.restart_server().await?;
    assert_eq!(letter_state(&served, &waiting).await?.state, "pending");
    let _woken = served.clock.advance(Duration::from_secs(10));
    let refused = claim(&served, &seats.a.bearer, &waiting, "k-restart-claim").await?;
    assert_eq!(refused.status, 409, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("mailbox-claim-reassigned"));
    served.stop().await
}
