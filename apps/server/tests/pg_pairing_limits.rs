use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use std::time::Duration;

use serde_json::json;

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::n106::invite_of;
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::two_step_client::{
    challenge_of, enable_two_step, fake_now_ms, signed_post, step_two_walked, wrong_code_for,
};
use anybot_server::KEY_TRUST_PROXY;
use anybot_server_test::computers_support::{
    RedeemAttempt, api_get, code_text, computer_rows, make_code_fast, other_code_than, redeem,
};
use anybot_server_test::{make_admin, register_body};

#[tokio::test]
async fn t_computers_pair_4() -> Result<(), String> {
    let mut served = serve_start("t_computers_pair_4").await?;
    let phone = make_admin(&served).await?;
    served
        .set_config_and_restart(KEY_TRUST_PROXY, "127.0.0.1")
        .await?;
    let wrong = "000000";
    for host in 1..=25 {
        let address = format!("10.0.0.{host}");
        for round in 1..=8 {
            let key = format!("w-{host}-{round}");
            let attempt = RedeemAttempt {
                code: wrong,
                name: "L",
                key: &key,
                address: Some(&address),
            };
            let answer = redeem(&served, &attempt).await?;
            assert_eq!(
                answer.status, 400,
                "{address} round {round}: {}",
                answer.json
            );
        }
    }
    let code = code_text(&served, &phone, "k-code").await?;
    for (index, address) in ["10.1.1.1", "10.0.0.1"].into_iter().enumerate() {
        let key = format!("blocked-{index}");
        let attempt = RedeemAttempt {
            code: &code,
            name: "L",
            key: &key,
            address: Some(address),
        };
        let blocked = redeem(&served, &attempt).await?;
        assert_eq!(blocked.status, 429, "{address}: {}", blocked.json);
        assert_eq!(blocked.code().as_deref(), Some("computers-pair-rate"));
    }
    let _woken = served.clock.advance(Duration::from_secs(301));
    let later = RedeemAttempt {
        code: &code,
        name: "L",
        key: "later",
        address: Some("10.1.1.1"),
    };
    let paired = redeem(&served, &later).await?;
    assert_eq!(paired.status, 200, "five minutes later: {}", paired.json);
    pending_cap(&served, &phone).await?;
    served.stop().await?;
    Ok(())
}

async fn pending_cap(served: &Served, phone: &str) -> Result<(), String> {
    for number in 1..=10_000 {
        let made = make_code_fast(served, phone, &format!("cap-{number}")).await?;
        assert_eq!(made.status, 200, "code {number}: {}", made.json);
    }
    let over = make_code_fast(served, phone, "cap-over").await?;
    assert_eq!(over.status, 429, "{}", over.json);
    assert_eq!(over.code().as_deref(), Some("computers-pair-full"));
    Ok(())
}

#[tokio::test]
async fn t_computers_pair_9() -> Result<(), String> {
    let served = serve_start("t_computers_pair_9").await?;
    let phone = make_admin(&served).await?;
    let code = code_text(&served, &phone, "k-code").await?;
    let first = RedeemAttempt {
        code: &code,
        name: "L",
        key: "same-key",
        address: None,
    };
    let lost = redeem(&served, &first).await?;
    assert_eq!(lost.status, 200, "{}", lost.json);
    let resent = redeem(&served, &first).await?;
    assert_eq!(resent.status, 200, "{}", resent.json);
    assert_eq!(resent.field_str("device_id")?, lost.field_str("device_id")?);
    assert_ne!(resent.bearer()?, lost.bearer()?);
    let used = api_get(&served, &resent.bearer()?, "/v1/computers").await?;
    assert_eq!(used.status, 200, "{}", used.json);
    let old = api_get(&served, &lost.bearer()?, "/v1/computers").await?;
    assert_eq!(
        old.status, 401,
        "the lost credential still works: {}",
        old.json
    );
    assert_eq!(computer_rows(&served, &phone).await?.len(), 1);
    let after_use = redeem(&served, &first).await?;
    assert_eq!(after_use.status, 400, "{}", after_use.json);
    let plain = RedeemAttempt {
        code: other_code_than(&code),
        name: "L",
        key: "plain",
        address: None,
    };
    let wrong = redeem(&served, &plain).await?;
    assert_eq!(after_use.json, wrong.json);
    let second_code = code_text(&served, &phone, "k-code-2").await?;
    let second = RedeemAttempt {
        code: &second_code,
        name: "M",
        key: "other-key",
        address: None,
    };
    let dropped = redeem(&served, &second).await?;
    assert_eq!(dropped.status, 200, "{}", dropped.json);
    let _woken = served.clock.advance(Duration::from_secs(601));
    let late = redeem(&served, &second).await?;
    assert_eq!(late.json, wrong.json);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_safety_n10_3() -> Result<(), String> {
    let served = serve_start("t_safety_n10_3").await?;
    let invite = invite_of(&served.started)?;
    for number in 1..=6 {
        let body = register_body(&format!("user{number}"), "userpassword", &invite);
        let answer = post(
            &served.base_url(),
            "/v1/account/register",
            &RequestSpec::new(&body).with_key(&format!("reg-{number}")),
        )
        .await?;
        let expected = if number == 6 { 429 } else { 201 };
        assert_eq!(
            answer.status, expected,
            "register {number}: {}",
            answer.json
        );
    }
    let user = enable_two_step(&served.base_url(), &served.clock, "user1", "userpassword").await?;
    let phone = user.bearer.clone();
    let code = code_text(&served, &phone, "k-code").await?;
    let wrong = other_code_than(&code);
    for round in 1..=9 {
        let key = format!("pair-{round}");
        let attempt = RedeemAttempt {
            code: wrong,
            name: "L",
            key: &key,
            address: None,
        };
        let answer = redeem(&served, &attempt).await?;
        let expected = if round == 9 { 429 } else { 400 };
        assert_eq!(
            answer.status, expected,
            "pairing round {round}: {}",
            answer.json
        );
    }
    two_step_rounds(&served, &user).await?;
    password_rounds(&served, &phone).await?;
    served.stop().await?;
    Ok(())
}

async fn two_step_rounds(
    served: &Served,
    user: &anybot_app_test::two_step_client::TwoStepUser,
) -> Result<(), String> {
    let base = served.base_url();
    let challenge = challenge_of(&base, user, "ts-login").await?;
    let bad = wrong_code_for(&user.authenticator, fake_now_ms(&served.clock))?;
    for round in 1..=4 {
        let answer = step_two_walked(
            &base,
            &served.clock,
            &challenge,
            &bad,
            &format!("ts-{round}"),
        )
        .await?;
        let rejected_as_spent = round == 4;
        assert_eq!(
            answer.code().as_deref() == Some("account-two-step-expired"),
            rejected_as_spent,
            "two-step round {round}: {}",
            answer.json
        );
    }
    Ok(())
}

async fn password_rounds(served: &Served, phone: &str) -> Result<(), String> {
    let mut old = "userpassword".to_owned();
    for round in 1..=9 {
        let next = format!("changed-password-{round}");
        let body = json!({"old": old, "new": next});
        let answer = signed_post(
            &served.base_url(),
            phone,
            "/v1/account/password",
            &body,
            &format!("pw-{round}"),
        )
        .await?;
        let expected = if round == 9 { 429 } else { 200 };
        assert_eq!(
            answer.status, expected,
            "password round {round}: {}",
            answer.json
        );
        old = next;
    }
    Ok(())
}
