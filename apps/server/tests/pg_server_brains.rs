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

use serde_json::json;

use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::log_capture::capture_server_logs;
use anybot_app_test::mailbox_prop::{Seat, make_bot};
use anybot_app_test::n106::{invite_of, login_spec, register};
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::sse::open_sse;
use anybot_app_test::{dump_database, found_spelling};
use anybot_server_test::computers_support::{api_post, computer_rows, row_of};
use anybot_server_test::make_admin;
use anybot_store_server::runner_device;

async fn runner_seat(served: &Served) -> Result<Seat, String> {
    let keys = served.db.secret_keys()?;
    let opened = runner_device::open_key(&served.pool, &keys)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    let row = runner_device::find(&served.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    Ok(Seat {
        bearer: opened.expose().to_owned(),
        device_id: row.device_id,
    })
}

async fn device_count(served: &Served) -> Result<i64, String> {
    sqlx::query_scalar("SELECT count(*) FROM runner_service_device")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())
}

async fn restarts_keep_one_device(served: &mut Served, first: &Seat) -> Result<(), String> {
    for _round in 0..2 {
        served.stop_server().await?;
        served.restart_server().await?;
        assert_eq!(device_count(served).await?, 1);
        let again = runner_seat(served).await?;
        assert_eq!(again.device_id, first.device_id);
        assert_eq!(again.bearer, first.bearer);
    }
    Ok(())
}

async fn concurrent_starts_make_one_device(served: &Served) -> Result<Seat, String> {
    sqlx::query("DELETE FROM runner_service_device")
        .execute(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    let mut starts = Vec::new();
    for _copy in 0..12 {
        let state = served.state.clone();
        starts.push(tokio::spawn(async move {
            anybot_server::runner_device::ensure_runner_device(&state).await
        }));
    }
    for start in starts {
        start.await.map_err(|e| e.to_string())??;
    }
    assert_eq!(device_count(served).await?, 1);
    runner_seat(served).await
}

async fn runner_claims_a_letter(served: &Served, phone: &str, runner: &Seat) -> Result<(), String> {
    make_bot(served, phone, "bot-brains", "k-brains-bot").await?;
    let mut sse = open_sse(
        &served.base_url(),
        "/v1/streams/notifications",
        &runner.bearer,
        "",
    )
    .await?;
    let body = json!({"bot": "bot-brains"});
    let handed = api_post(served, phone, "/v1/mailbox/round", &body, "k-brains-round").await?;
    assert_eq!(handed.status, 200, "{}", handed.json);
    assert_eq!(handed.field_str("device_id")?, runner.device_id);
    let letter = handed.field_str("letter_id")?;
    let hint = sse.recv_json_of("mail").await?;
    assert_eq!(
        hint.get("letter_id").and_then(|v| v.as_str()),
        Some(letter.as_str())
    );
    let claim = RequestSpec::new(&json!({"letter_id": letter}).to_string())
        .with_key("k-brains-claim")
        .with_bearer(&runner.bearer);
    let claimed = post(&served.base_url(), "/v1/mailbox/claim", &claim).await?;
    assert_eq!(claimed.status, 200, "{}", claimed.json);
    assert_eq!(claimed.field_str("kind")?, "round");
    sse.close();
    Ok(())
}

async fn lists_do_not_show_the_runner(
    served: &Served,
    alice: &str,
    bob: &str,
    runner: &Seat,
) -> Result<(), String> {
    for bearer in [alice, bob] {
        let rows = computer_rows(served, bearer).await?;
        assert!(
            row_of(&rows, &runner.device_id).is_none(),
            "the runner is on a computer list"
        );
    }
    Ok(())
}

async fn runner_is_refused_admin_and_pairing(served: &Served, runner: &Seat) -> Result<(), String> {
    let base = served.base_url();
    let spec = RequestSpec::new("{}")
        .with_key("k-brains-denied")
        .with_bearer(&runner.bearer);
    let invite = get(&base, "/v1/admin/invite", &spec).await?;
    assert_eq!(invite.status, 403, "{}", invite.json);
    let pairing = post(&base, "/v1/computers/pairing-code", &spec).await?;
    assert_eq!(pairing.status, 403, "{}", pairing.json);
    assert_eq!(pairing.code().as_deref(), Some("device-no-permission"));
    let rotate = post(&base, "/v1/admin/invite/rotate", &spec).await?;
    assert_eq!(rotate.status, 403, "{}", rotate.json);
    Ok(())
}

#[tokio::test]
async fn t_server_brains_7() -> Result<(), String> {
    let logs = capture_server_logs()?;
    let mut served = serve_start("t_server_brains_7").await?;
    let invite = invite_of(&served.started)?;
    let alice = make_admin(&served).await?;
    register(
        &served.base_url(),
        "bob",
        "bobpassword",
        &invite,
        "k-bob-reg",
    )
    .await?;
    let bob_login = post(
        &served.base_url(),
        "/v1/account/login",
        &login_spec("bob", "bobpassword", "k-bob-login", "bob-address"),
    )
    .await?;
    assert_eq!(bob_login.status, 200, "{}", bob_login.json);
    let bob = bob_login.bearer()?;
    assert_eq!(device_count(&served).await?, 1);
    let first = runner_seat(&served).await?;
    restarts_keep_one_device(&mut served, &first).await?;
    let runner = concurrent_starts_make_one_device(&served).await?;
    runner_claims_a_letter(&served, &alice, &runner).await?;
    lists_do_not_show_the_runner(&served, &alice, &bob, &runner).await?;
    runner_is_refused_admin_and_pairing(&served, &runner).await?;
    let dump = dump_database(&served.db)?;
    assert_eq!(found_spelling(&dump, runner.bearer.as_bytes()), None);
    assert!(
        logs.lines()
            .iter()
            .all(|line| !line.contains(&runner.bearer)),
        "the runner credential reached the server log"
    );
    served.stop().await
}
