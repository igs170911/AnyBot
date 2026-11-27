use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
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

use serde_json::json;

use anybot_app_test::ClockWalker;
use anybot_app_test::http::post;
use anybot_app_test::n106::{invite_of, register};
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::two_step_client::{
    TWO_STEP_SETUP_PATH, TwoStepUser, challenge_of, enable_two_step, fake_now_ms, signed_post,
    step_two_spec,
};
use anybot_app_test::{dump_database, found_digest, found_raw_bytes, found_spelling};

const PROXY_WORD: &str = "dump-check-proxy-word-41c2";

struct Seen {
    tokens: Vec<String>,
    texts: Vec<String>,
    bodies: Vec<String>,
    secrets: Vec<Vec<u8>>,
}

async fn change_password(served: &Served, bearer: &str) -> Result<(), String> {
    let mut walker = ClockWalker::start(&served.clock);
    let body = json!({"old": "alicepassword", "new": "alicechanged1"});
    let answer = signed_post(
        &served.base_url(),
        bearer,
        "/v1/account/password",
        &body,
        "pw",
    )
    .await;
    walker.stop().await?;
    let answer = answer?;
    assert_eq!(answer.status, 200, "password change: {}", answer.json);
    Ok(())
}

async fn second_step_logins(
    served: &Served,
    user: &TwoStepUser,
    seen: &mut Seen,
) -> Result<(), String> {
    let base = served.base_url();
    let code = user.authenticator.code_at(fake_now_ms(&served.clock))?;
    let recovery = user
        .recovery_codes
        .first()
        .ok_or("no recovery code")?
        .clone();
    for (round, typed) in [code, recovery].into_iter().enumerate() {
        let challenge = challenge_of(&base, user, &format!("dump-{round}")).await?;
        let spec = step_two_spec(&challenge, &typed, &format!("dump-step-{round}"));
        let answer = post(
            &base,
            anybot_app_test::two_step_client::TWO_STEP_LOGIN_PATH,
            &spec,
        )
        .await?;
        assert_eq!(answer.status, 200, "second step {round}: {}", answer.json);
        seen.tokens.extend([challenge, answer.bearer()?]);
        seen.bodies.push(spec.body.clone());
    }
    Ok(())
}

async fn pending_setup(served: &Served, user: &TwoStepUser, seen: &mut Seen) -> Result<(), String> {
    let code = user
        .authenticator
        .previous_code_at(fake_now_ms(&served.clock))?;
    let body = json!({"code": code});
    let setup = signed_post(
        &served.base_url(),
        &user.bearer,
        TWO_STEP_SETUP_PATH,
        &body,
        "dump-swap",
    )
    .await?;
    assert_eq!(setup.status, 200, "pending setup: {}", setup.json);
    let pending = setup.field_str("secret")?;
    let decoded = secret_from_base32(&pending)?;
    seen.secrets.push(decoded);
    seen.texts.push(pending);
    let codes = anybot_app_test::two_step_client::string_list(&setup.json, "recovery_codes")?;
    seen.texts.extend(codes);
    seen.bodies.push(body.to_string());
    Ok(())
}

fn secret_from_base32(text: &str) -> Result<Vec<u8>, String> {
    anybot_app_test::fake_totp::FakeAuthenticator::from_base32(text)
        .map(|found| found.secret().to_vec())
}

async fn run_every_action(served: &mut Served) -> Result<Seen, String> {
    let invite = invite_of(&served.started)?;
    register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-alice",
    )
    .await?;
    drop(invite);
    let user = enable_two_step(&served.base_url(), &served.clock, "alice", "alicepassword").await?;
    let mut seen = Seen {
        tokens: vec![user.bearer.clone()],
        texts: vec!["alicepassword".to_owned(), user.secret_base32.clone()],
        bodies: Vec::new(),
        secrets: vec![
            user.authenticator.secret().to_vec(),
            served.db.master_key.expose_bytes().to_vec(),
        ],
    };
    seen.texts.extend(user.recovery_codes.iter().cloned());
    served.set_config_and_restart(anybot_server::KEY_PROXY_SECRET, PROXY_WORD).await?;
    seen.texts.push(PROXY_WORD.to_owned());
    second_step_logins(served, &user, &mut seen).await?;
    pending_setup(served, &user, &mut seen).await?;
    change_password(served, &user.bearer).await?;
    seen.texts.push("alicechanged1".to_owned());
    Ok(seen)
}

fn assert_absent(dump: &str, seen: &Seen) {
    for raw in &seen.secrets {
        assert!(!found_raw_bytes(dump, raw), "raw secret bytes in the dump");
        assert_eq!(found_spelling(dump, raw), None, "secret bytes in the dump");
    }
    for token in &seen.tokens {
        assert_eq!(
            found_spelling(dump, token.as_bytes()),
            None,
            "token {token} in the dump"
        );
    }
    for text in &seen.texts {
        assert_eq!(
            found_spelling(dump, text.as_bytes()),
            None,
            "value {text} in the dump"
        );
        assert_eq!(
            found_digest(dump, text.as_bytes()),
            None,
            "digest of {text} in the dump"
        );
    }
    for body in &seen.bodies {
        assert_eq!(
            found_digest(dump, body.as_bytes()),
            None,
            "digest of the body {body}"
        );
    }
}

#[tokio::test]
async fn every_secret_value_is_absent_from_a_full_dump() -> Result<(), String> {
    let mut served = serve_start("secrets_dump_all").await?;
    let seen = run_every_action(&mut served).await?;
    let dump = dump_database(&served.db)?;
    assert_absent(&dump, &seen);
    served.stop().await
}
