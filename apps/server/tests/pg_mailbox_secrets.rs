use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use tracing as _;

mod pg_mailbox_shared;

use anybot_app_test::log_capture::capture_server_logs;
use anybot_app_test::mailbox_prop::Seats;
use anybot_app_test::serve::Served;
use anybot_app_test::serve::serve_start;
use anybot_app_test::{dump_database, found_digest, found_spelling};
use anybot_server_test::computers_support::{RedeemAttempt, code_text, redeem};
use anybot_server_test::mailbox_support::{claim, deliver};
use pg_mailbox_shared::three_stand_ins;

async fn use_pairing(served: &Served, seats: &Seats) -> Result<Vec<String>, String> {
    let used_code = code_text(served, &seats.phone, "k-code-1").await?;
    let attempt = RedeemAttempt {
        code: &used_code,
        name: "P",
        key: "r-1",
        address: None,
    };
    let paired = redeem(served, &attempt).await?;
    assert_eq!(paired.status, 200, "{}", paired.json);
    let open_code = code_text(served, &seats.phone, "k-code-2").await?;
    Ok(vec![used_code, open_code, paired.bearer()?])
}

async fn use_mailbox(served: &Served, seats: &Seats) -> Result<(), String> {
    let bot = "bot-secrets";
    let computer = Some(seats.a.device_id.as_str());
    let handed = deliver(served, &seats.phone, bot, computer, "k-s-round").await?;
    let letter = handed.field_str("letter_id")?;
    let claimed = claim(served, &seats.a.bearer, &letter, "k-s-claim").await?;
    assert_eq!(claimed.status, 200, "{}", claimed.json);
    let to_bot = deliver(served, &seats.phone, bot, None, "k-s-bot").await?;
    let bot_letter = to_bot.field_str("letter_id")?;
    let by_runner = claim(served, &seats.x.bearer, &bot_letter, "k-s-runner").await?;
    assert_eq!(by_runner.status, 200, "{}", by_runner.json);
    Ok(())
}

#[tokio::test]
async fn no_secret_from_pairing_login_runner_or_mailbox_is_stored_or_logged_readable()
-> Result<(), String> {
    let logs = capture_server_logs()?;
    let served = serve_start("mailbox_secrets").await?;
    let seats = three_stand_ins(&served, "bot-secrets").await?;
    let mut secrets = use_pairing(&served, &seats).await?;
    use_mailbox(&served, &seats).await?;
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM idempotency")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert!(kept > 0, "no idempotency records to check");
    let codes: Vec<String> = secrets.iter().take(2).cloned().collect();
    secrets.extend([
        seats.phone.clone(),
        seats.a.bearer.clone(),
        seats.b.bearer.clone(),
        seats.x.bearer.clone(),
    ]);
    let dump = dump_database(&served.db)?;
    let lines = logs.lines();
    for secret in &secrets {
        assert_eq!(
            found_spelling(&dump, secret.as_bytes()),
            None,
            "a secret is in the dump"
        );
        assert!(
            lines.iter().all(|line| !line.contains(secret.as_str())),
            "a secret reached the server log"
        );
    }
    for code in &codes {
        assert_eq!(
            found_digest(&dump, code.as_bytes()),
            None,
            "unkeyed digest of a pairing code"
        );
    }
    served.stop().await
}
