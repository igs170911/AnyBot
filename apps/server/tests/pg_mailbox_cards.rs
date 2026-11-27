use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use tokio as _;
use tracing as _;

mod pg_mailbox_shared;

use std::time::Duration;

use anybot_app_test::mailbox_prop::Seats;
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::sse::open_sse;
use anybot_server_test::computers_support::api_post;
use anybot_server_test::mailbox_support::{claim, deliver, letter_state};
use anybot_store_server::mailbox::{NewLetter, insert};
use pg_mailbox_shared::three_stand_ins;

const BOT: &str = "bot-cards";
const SETTLE: Duration = Duration::from_millis(400);

async fn account_of(served: &Served, device_id: &str) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT account FROM computers WHERE device_id = $1")
        .bind(device_id)
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())
}

async fn put_card_letter(served: &Served, seats: &Seats, letter_id: &str) -> Result<(), String> {
    let account = account_of(served, &seats.a.device_id).await?;
    let fresh = NewLetter {
        letter_id,
        account: &account,
        kind: "card-answer",
        device_id: &seats.a.device_id,
        content: "{\"card\":\"c1\",\"answer\":\"yes\"}",
        now_ms: i64::try_from(served.clock.now_ms()).map_err(|e| e.to_string())?,
    };
    insert(&served.pool, &fresh)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::test]
async fn card_answer_letters_outlive_ten_seconds_and_are_retracted_when_the_computer_goes()
-> Result<(), String> {
    let served = serve_start("mailbox_cards").await?;
    let seats = three_stand_ins(&served, BOT).await?;
    put_card_letter(&served, &seats, "card-kept").await?;
    put_card_letter(&served, &seats, "card-dropped").await?;
    let _woken = served.clock.advance(Duration::from_secs(3600));
    test_harness::sleep_real(SETTLE).await;
    assert_eq!(letter_state(&served, "card-kept").await?.state, "pending");
    assert_eq!(
        letter_state(&served, "card-dropped").await?.state,
        "pending"
    );
    let kept = claim(&served, &seats.a.bearer, "card-kept", "k-card-claim").await?;
    assert_eq!(kept.status, 200, "{}", kept.json);
    assert_eq!(kept.field_str("kind")?, "card-answer");
    let body = serde_json::json!({"device_id": seats.a.device_id});
    let removed = api_post(
        &served,
        &seats.b.bearer,
        "/v1/computers/remove",
        &body,
        "k-card-rm",
    )
    .await?;
    assert_eq!(removed.status, 200, "{}", removed.json);
    assert_eq!(
        letter_state(&served, "card-dropped").await?.state,
        "retracted"
    );
    assert_eq!(letter_state(&served, "card-kept").await?.state, "claimed");
    served.stop().await
}

#[tokio::test]
async fn a_mail_hint_names_the_letter_only_and_marks_it_delivered() -> Result<(), String> {
    let served = serve_start("mailbox_hint").await?;
    let seats = three_stand_ins(&served, BOT).await?;
    let to_bot = deliver(&served, &seats.phone, BOT, None, "k-hint-x").await?;
    assert_eq!(to_bot.status, 200, "{}", to_bot.json);
    let letter = to_bot.field_str("letter_id")?;
    let to_a = deliver(
        &served,
        &seats.phone,
        BOT,
        Some(&seats.a.device_id),
        "k-hint-a",
    )
    .await?;
    let letter_a = to_a.field_str("letter_id")?;
    let mut stream = open_sse(
        &served.base_url(),
        "/v1/streams/notifications",
        &seats.x.bearer,
        "",
    )
    .await?;
    let hint = stream.recv_json_of("mail").await?;
    let mut keys: Vec<&str> = hint
        .as_object()
        .map(|fields| fields.keys().map(String::as_str).collect())
        .unwrap_or_default();
    keys.sort_unstable();
    assert_eq!(keys, vec!["kind", "letter_id"], "{hint}");
    assert_eq!(
        hint.get("letter_id").and_then(|v| v.as_str()),
        Some(letter.as_str())
    );
    let mut marked = false;
    for _round in 0..50 {
        if letter_state(&served, &letter)
            .await?
            .delivered_at_ms
            .is_some()
        {
            marked = true;
            break;
        }
        test_harness::sleep_real(Duration::from_millis(100)).await;
    }
    assert!(marked, "the hint did not record the delivery");
    assert_eq!(
        letter_state(&served, &letter_a).await?.delivered_at_ms,
        None
    );
    stream.close();
    served.stop().await
}
