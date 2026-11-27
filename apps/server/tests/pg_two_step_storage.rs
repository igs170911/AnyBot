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

use anybot_app_test::log_capture::capture_server_logs;
use anybot_app_test::n106::{invite_of, register};
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::two_step_client::{
    TwoStepUser, challenge_of, enable_two_step, fake_now_ms, step_two,
};
use anybot_app_test::{dump_database, found_digest, found_raw_bytes, found_spelling};

async fn served_with(test: &str, accounts: &[&str]) -> Result<(Served, Vec<TwoStepUser>), String> {
    let served = serve_start(test).await?;
    let invite = invite_of(&served.started)?;
    let mut users = Vec::new();
    for account in accounts {
        let password = format!("{account}password");
        register(
            &served.base_url(),
            account,
            &password,
            &invite,
            &format!("k-{account}"),
        )
        .await?;
        users.push(enable_two_step(&served.base_url(), &served.clock, account, &password).await?);
    }
    Ok((served, users))
}

async fn login_with_code_and_recovery(served: &Served, user: &TwoStepUser) -> Result<(), String> {
    let base = served.base_url();
    let challenge = challenge_of(&base, user, "store-a").await?;
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let passed = step_two(&base, &challenge, &code, "store-b").await?;
    assert_eq!(passed.status, 200, "code login: {}", passed.json);
    let recovery = user.recovery_codes.first().ok_or("no recovery code")?;
    let challenge = challenge_of(&base, user, "store-c").await?;
    let passed = step_two(&base, &challenge, recovery, "store-d").await?;
    assert_eq!(passed.status, 200, "recovery login: {}", passed.json);
    Ok(())
}

fn assert_no_two_step_secret(dump: &str, user: &TwoStepUser) {
    let secret = user.authenticator.secret();
    assert_eq!(secret.len(), 20);
    assert!(
        !found_raw_bytes(dump, secret),
        "raw shared secret in the dump"
    );
    assert_eq!(
        found_spelling(dump, secret),
        None,
        "shared secret in the dump"
    );
    assert!(
        !dump.contains(&user.secret_base32),
        "otpauth base32 secret in the dump"
    );
    assert_eq!(user.recovery_codes.len(), 8);
    for code in &user.recovery_codes {
        assert_eq!(
            found_spelling(dump, code.as_bytes()),
            None,
            "recovery code {code}"
        );
        assert_eq!(
            found_digest(dump, code.as_bytes()),
            None,
            "unkeyed digest of {code}"
        );
    }
}

#[tokio::test]
async fn t_account_n3_5() -> Result<(), String> {
    let (served, users) = served_with("t_account_n3_5", &["alice"]).await?;
    let user = users.first().ok_or("no user")?;
    assert_no_two_step_secret(&dump_database(&served.db)?, user);
    login_with_code_and_recovery(&served, user).await?;
    assert_no_two_step_secret(&dump_database(&served.db)?, user);
    served.stop().await
}

#[tokio::test]
async fn t_server_privacy_8() -> Result<(), String> {
    let (served, users) = served_with("t_server_privacy_8", &["alice"]).await?;
    let user = users.first().ok_or("no user")?;
    login_with_code_and_recovery(&served, user).await?;
    let dump = dump_database(&served.db)?;
    let master = served.db.master_key.expose_bytes();
    assert!(
        !found_raw_bytes(&dump, master),
        "raw master key in the dump"
    );
    assert_eq!(
        found_spelling(&dump, master),
        None,
        "master key in the dump"
    );
    served.stop().await
}

async fn move_alice_secret_to_bob(served: &Served) -> Result<(), String> {
    let moved = sqlx::query(
        "UPDATE two_step SET secret_sealed = \
         (SELECT secret_sealed FROM two_step WHERE account = 'alice') WHERE account = 'bob'",
    )
    .execute(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    assert_eq!(moved.rows_affected(), 1);
    Ok(())
}

#[tokio::test]
async fn t_server_privacy_7() -> Result<(), String> {
    let logs = capture_server_logs()?;
    let (served, users) = served_with("t_server_privacy_7", &["alice", "bob"]).await?;
    let (alice, bob) = (
        users.first().ok_or("no alice")?,
        users.get(1).ok_or("no bob")?,
    );
    move_alice_secret_to_bob(&served).await?;
    let base = served.base_url();
    let now = fake_now_ms(&served.clock);
    for (round, code) in [
        alice.authenticator.code_at(now)?,
        bob.authenticator.code_at(now)?,
    ]
    .iter()
    .enumerate()
    {
        let challenge = challenge_of(&base, bob, &format!("moved-{round}")).await?;
        let answer = step_two(&base, &challenge, code, &format!("moved-step-{round}")).await?;
        assert_eq!(
            answer.status, 500,
            "moved secret round {round}: {}",
            answer.json
        );
        assert_eq!(
            answer.code().as_deref(),
            Some("account-two-step-unreadable")
        );
    }
    let lines = logs.lines();
    let reason = lines
        .iter()
        .find(|line| line.contains("two-step secret could not be opened"))
        .ok_or(format!("no log line names the failure: {lines:?}"))?;
    assert!(
        reason.contains("bob"),
        "log line names the account: {reason}"
    );
    assert!(
        reason.contains("moved to another table, row or column"),
        "log line gives the reason: {reason}"
    );
    served.stop().await
}
