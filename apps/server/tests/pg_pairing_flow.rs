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

use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::{
    RedeemAttempt, api_get, code_text, computer_login, computer_rows, make_code, name_of,
    other_code_than, reauth, redeem, redeem_timed,
};
use anybot_server_test::make_admin;

const WORK_MACHINE: &str = "\u{5de5}\u{4f5c}\u{6a5f}";
const SERVER_ROOM: &str = "\u{6a5f}\u{623f}";

#[tokio::test]
async fn t_computers_pair_2() -> Result<(), String> {
    let served = serve_start("t_computers_pair_2").await?;
    let phone = make_admin(&served).await?;
    let _woken = served.clock.advance(Duration::from_mins(16));
    let refused = make_code(&served, &phone, "k-1").await?;
    assert_eq!(refused.status, 403, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("account-reauth-required"));
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pairing_codes")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(pending, 0, "a code was made without the password");
    let wrong = reauth(&served, &phone, "not-the-password", "k-2").await?;
    assert_eq!(wrong.status, 403, "{}", wrong.json);
    let entered = reauth(&served, &phone, "alicepassword", "k-3").await?;
    assert_eq!(entered.status, 200, "{}", entered.json);
    let made = make_code(&served, &phone, "k-4").await?;
    assert_eq!(made.status, 200, "{}", made.json);
    let code = made.field_str("code")?;
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()), "{code}");
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_pair_3() -> Result<(), String> {
    let served = serve_start("t_computers_pair_3").await?;
    let phone = make_admin(&served).await?;
    let code = code_text(&served, &phone, "k-code").await?;
    let wrong = other_code_than(&code);
    for round in 1..=8 {
        let key = format!("w-{round}");
        let attempt = RedeemAttempt {
            code: wrong,
            name: "L",
            key: &key,
            address: None,
        };
        let (answer, waited_ms) = redeem_timed(&served, &attempt).await?;
        assert_eq!(answer.status, 400, "round {round}: {}", answer.json);
        assert_eq!(answer.code().as_deref(), Some("computers-pair-wrong"));
        assert!(
            waited_ms >= 300,
            "round {round} answered after {waited_ms} ms"
        );
    }
    let attempt = RedeemAttempt {
        code: &code,
        name: "L",
        key: "w-9",
        address: None,
    };
    let (ninth, waited_ms) = redeem_timed(&served, &attempt).await?;
    assert_eq!(ninth.status, 429, "{}", ninth.json);
    assert_eq!(ninth.code().as_deref(), Some("computers-pair-rate"));
    assert!(waited_ms >= 300, "blocked answer after {waited_ms} ms");
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_pair_5() -> Result<(), String> {
    let mut served = serve_start("t_computers_pair_5").await?;
    let phone = make_admin(&served).await?;
    let code = code_text(&served, &phone, "k-code").await?;
    served.stop_server().await?;
    served.restart_server().await?;
    let first = RedeemAttempt {
        code: &code,
        name: "L",
        key: "r-1",
        address: None,
    };
    let paired = redeem(&served, &first).await?;
    assert_eq!(paired.status, 200, "{}", paired.json);
    let again = RedeemAttempt {
        code: &code,
        name: "L",
        key: "r-2",
        address: None,
    };
    let second = redeem(&served, &again).await?;
    assert_eq!(second.status, 400, "{}", second.json);
    assert_eq!(second.code().as_deref(), Some("computers-pair-wrong"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_pair_6() -> Result<(), String> {
    let served = serve_start("t_computers_pair_6").await?;
    let phone = make_admin(&served).await?;
    let code = code_text(&served, &phone, "k-code").await?;
    let _woken = served.clock.advance(Duration::from_secs(601));
    let attempt = RedeemAttempt {
        code: &code,
        name: "L",
        key: "r-1",
        address: None,
    };
    let late = redeem(&served, &attempt).await?;
    assert_eq!(late.status, 400, "{}", late.json);
    assert_eq!(late.code().as_deref(), Some("computers-pair-wrong"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_computers_pair_7() -> Result<(), String> {
    let served = serve_start("t_computers_pair_7").await?;
    let phone = make_admin(&served).await?;
    let work = computer_login(&served, WORK_MACHINE, "k-work").await?;
    let room = computer_login(&served, SERVER_ROOM, "k-room").await?;
    let _woken = served.clock.advance(Duration::from_secs(61));
    let alive = api_get(&served, &work.bearer, "/v1/computers").await?;
    assert_eq!(alive.status, 200, "{}", alive.json);
    let rows = computer_rows(&served, &phone).await?;
    let offline_flags: Vec<bool> = rows
        .iter()
        .filter_map(|row| row.get("online").and_then(serde_json::Value::as_bool))
        .collect();
    assert_eq!(
        offline_flags.iter().filter(|online| !**online).count(),
        1,
        "{rows:?}"
    );
    for (round, name) in [WORK_MACHINE, SERVER_ROOM].into_iter().enumerate() {
        let code = code_text(&served, &phone, &format!("k-code-{round}")).await?;
        let key = format!("r-{round}");
        let attempt = RedeemAttempt {
            code: &code,
            name,
            key: &key,
            address: None,
        };
        let paired = redeem(&served, &attempt).await?;
        assert_eq!(paired.status, 200, "{}", paired.json);
    }
    let rows = computer_rows(&served, &phone).await?;
    let mut names: Vec<&str> = rows.iter().filter_map(name_of).collect();
    names.sort_unstable();
    let mut expected = vec![
        WORK_MACHINE,
        SERVER_ROOM,
        "\u{5de5}\u{4f5c}\u{6a5f} (2)",
        "\u{6a5f}\u{623f} (2)",
    ];
    expected.sort_unstable();
    assert_eq!(names, expected);
    for computer in [&work, &room] {
        let still = api_get(&served, &computer.bearer, "/v1/computers").await?;
        assert_eq!(still.status, 200, "{}", still.json);
    }
    served.stop().await?;
    Ok(())
}
