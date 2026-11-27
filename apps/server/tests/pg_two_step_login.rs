use anybot_app_server as _;
use anybot_domain as _;
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

use std::time::Duration;

use serde_json::json;

use anybot_app_test::n106::{invite_of, register};
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::two_step_client::{
    TWO_STEP_DISABLE_PATH, TWO_STEP_LOGIN_PATH, TWO_STEP_SETUP_PATH, TwoStepUser, challenge_of,
    enable_two_step, fake_now_ms, signed_post, step_two, step_two_spec, step_two_walked,
    timed_post, wrong_code_for,
};

const ALICE_PASSWORD: &str = "alicepassword";

async fn alice_with_two_step(test: &str) -> Result<(Served, TwoStepUser), String> {
    let served = serve_start(test).await?;
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "alice",
        ALICE_PASSWORD,
        &invite,
        "k-alice",
    )
    .await?;
    let user = enable_two_step(&served.base_url(), &served.clock, "alice", ALICE_PASSWORD).await?;
    Ok((served, user))
}

async fn pending_and_recovery(served: &Served) -> Result<(bool, i64), String> {
    let pending: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT pending_sealed FROM two_step WHERE account = 'alice'")
            .fetch_one(&served.pool)
            .await
            .map_err(|e| e.to_string())?;
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM two_step_recovery WHERE account = 'alice' AND NOT pending",
    )
    .fetch_one(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok((pending.is_some(), active))
}

#[tokio::test]
async fn t_account_n3_2() -> Result<(), String> {
    let (served, user) = alice_with_two_step("t_account_n3_2").await?;
    let base = served.base_url();
    let setup = signed_post(
        &base,
        &user.bearer,
        TWO_STEP_SETUP_PATH,
        &json!({}),
        "swap-no-code",
    )
    .await?;
    assert_eq!(setup.status, 401, "swap without a code: {}", setup.json);
    assert_eq!(
        setup.code().as_deref(),
        Some("account-two-step-setup-code-wrong")
    );
    let off = signed_post(
        &base,
        &user.bearer,
        TWO_STEP_DISABLE_PATH,
        &json!({}),
        "off-no-code",
    )
    .await?;
    assert_eq!(off.status, 401, "disable without a code: {}", off.json);
    assert_eq!(
        off.code().as_deref(),
        Some("account-two-step-setup-code-wrong")
    );
    assert_eq!(pending_and_recovery(&served).await?, (false, 8));
    let challenge = challenge_of(&base, &user, "still-on").await?;
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let passed = step_two(&base, &challenge, &code, "still-on-2").await?;
    assert_eq!(
        passed.status, 200,
        "the old authenticator still works: {}",
        passed.json
    );
    served.stop().await
}

fn align_to_step_start(served: &Served) -> Result<(), String> {
    let now = fake_now_ms(&served.clock);
    let into = now.checked_rem(30_000).ok_or("zero step")?;
    let gap = 31_000u64.checked_sub(into).ok_or("step gap")?;
    let _woken = served.clock.advance(Duration::from_millis(gap));
    Ok(())
}

async fn three_wrong_then_expired(served: &Served, user: &TwoStepUser) -> Result<(), String> {
    let base = served.base_url();
    let challenge = challenge_of(&base, user, "n3-3-a").await?;
    for round in 1..=3 {
        let wrong = wrong_code_for(&user.authenticator, fake_now_ms(&served.clock))?;
        let answer = step_two_walked(
            &base,
            &served.clock,
            &challenge,
            &wrong,
            &format!("w{round}"),
        )
        .await?;
        assert_eq!(answer.status, 401, "wrong code {round}: {}", answer.json);
        assert_eq!(
            answer.code().as_deref(),
            Some("account-two-step-code-wrong")
        );
    }
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let fourth = step_two(&base, &challenge, &code, "w4").await?;
    assert_eq!(fourth.status, 400, "after three tries: {}", fourth.json);
    assert_eq!(fourth.code().as_deref(), Some("account-two-step-expired"));
    Ok(())
}

async fn codes_and_reuse(served: &Served, user: &TwoStepUser) -> Result<(), String> {
    let base = served.base_url();
    align_to_step_start(served)?;
    let now = fake_now_ms(&served.clock);
    let challenge = challenge_of(&base, user, "n3-3-b").await?;
    let previous = user.authenticator.previous_code_at(now)?;
    let passed = step_two(&base, &challenge, &previous, "prev").await?;
    assert_eq!(passed.status, 200, "previous step code: {}", passed.json);
    let next = user.authenticator.next_code_at(now)?;
    let challenge = challenge_of(&base, user, "n3-3-c").await?;
    let passed = step_two(&base, &challenge, &next, "next").await?;
    assert_eq!(passed.status, 200, "next step code: {}", passed.json);
    let _woken = served.clock.advance(Duration::from_secs(60));
    let challenge = challenge_of(&base, user, "n3-3-d").await?;
    let reused = step_two_walked(&base, &served.clock, &challenge, &next, "reuse").await?;
    assert_eq!(
        reused.status, 401,
        "same code one minute later: {}",
        reused.json
    );
    assert_eq!(
        reused.code().as_deref(),
        Some("account-two-step-code-wrong")
    );
    Ok(())
}

async fn recovery_twice(served: &Served, user: &TwoStepUser) -> Result<(), String> {
    let base = served.base_url();
    let recovery = user.recovery_codes.first().ok_or("no recovery code")?;
    let challenge = challenge_of(&base, user, "n3-3-e").await?;
    let first = step_two(&base, &challenge, recovery, "rec-1").await?;
    assert_eq!(first.status, 200, "first recovery use: {}", first.json);
    let challenge = challenge_of(&base, user, "n3-3-f").await?;
    let second = step_two_walked(&base, &served.clock, &challenge, recovery, "rec-2").await?;
    assert_eq!(second.status, 401, "second recovery use: {}", second.json);
    assert_eq!(
        second.code().as_deref(),
        Some("account-two-step-code-wrong")
    );
    Ok(())
}

#[tokio::test]
async fn t_account_n3_3() -> Result<(), String> {
    let (served, user) = alice_with_two_step("t_account_n3_3").await?;
    three_wrong_then_expired(&served, &user).await?;
    codes_and_reuse(&served, &user).await?;
    recovery_twice(&served, &user).await?;
    served.stop().await
}

async fn timed_wrong(
    served: &Served,
    user: &TwoStepUser,
    challenge: &str,
    round: usize,
) -> Result<(), String> {
    let wrong = wrong_code_for(&user.authenticator, fake_now_ms(&served.clock))?;
    let spec = step_two_spec(challenge, &wrong, &format!("rate-{round}"));
    let (answer, waited) = timed_post(
        &served.base_url(),
        &served.clock,
        TWO_STEP_LOGIN_PATH,
        &spec,
    )
    .await?;
    assert_eq!(answer.status, 401, "wrong code {round}: {}", answer.json);
    assert_eq!(
        answer.code().as_deref(),
        Some("account-two-step-code-wrong")
    );
    assert!(
        waited >= 300,
        "wrong code {round} answered after {waited} ms"
    );
    Ok(())
}

#[tokio::test]
async fn t_account_n3_4() -> Result<(), String> {
    let (served, user) = alice_with_two_step("t_account_n3_4").await?;
    let base = served.base_url();
    let mut challenge = String::new();
    for round in 1..=8usize {
        if round.checked_rem(3) == Some(1) {
            challenge = challenge_of(&base, &user, &format!("rate-login-{round}")).await?;
        }
        timed_wrong(&served, &user, &challenge, round).await?;
    }
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let ninth = step_two(&base, &challenge, &code, "rate-9").await?;
    assert_eq!(ninth.status, 429, "ninth attempt: {}", ninth.json);
    assert_eq!(ninth.code().as_deref(), Some("account-two-step-rate"));
    let voided = step_two(&base, &challenge, &code, "rate-10").await?;
    assert_eq!(voided.code().as_deref(), Some("account-two-step-expired"));
    let _woken = served.clock.advance(Duration::from_secs(301));
    let fresh = challenge_of(&base, &user, "rate-later").await?;
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let later = step_two(&base, &fresh, &code, "rate-later-2").await?;
    assert_eq!(later.status, 200, "five minutes later: {}", later.json);
    served.stop().await
}
