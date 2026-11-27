use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::log_capture::capture_server_logs;
use anybot_app_test::serve::{Served, serve_start};
use anybot_app_test::sse::open_sse;
use anybot_app_test::{dump_database, found_spelling};
use anybot_server_test::computers_support::computer_login;
use anybot_server_test::make_admin;
use anybot_store_server::runner_device;

async fn device_count(served: &Served) -> Result<i64, String> {
    sqlx::query_scalar("SELECT count(*) FROM runner_service_device")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())
}

async fn runner_key(served: &Served) -> Result<String, String> {
    let keys = served.db.secret_keys()?;
    let opened = runner_device::open_key(&served.pool, &keys)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    Ok(opened.expose().to_owned())
}

async fn runner_device_id(served: &Served) -> Result<String, String> {
    let row = runner_device::find(&served.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no runner device")?;
    Ok(row.device_id)
}

#[tokio::test]
async fn runner_device_stays_single_across_restarts_and_concurrent_starts() -> Result<(), String> {
    let mut served = serve_start("runner_device_single").await?;
    assert_eq!(device_count(&served).await?, 1);
    let first_id = runner_device_id(&served).await?;
    let first_key = runner_key(&served).await?;
    for _round in 0..2 {
        served.stop_server().await?;
        served.restart_server().await?;
        assert_eq!(device_count(&served).await?, 1);
    }
    assert_eq!(runner_device_id(&served).await?, first_id);
    assert_eq!(runner_key(&served).await?, first_key);
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
    assert_eq!(device_count(&served).await?, 1);
    let winner = runner_key(&served).await?;
    assert_ne!(winner, first_key);
    served.stop().await
}

#[tokio::test]
async fn runner_credential_is_sealed_and_never_logged() -> Result<(), String> {
    let logs = capture_server_logs()?;
    let served = serve_start("runner_device_sealed").await?;
    let phone = make_admin(&served).await?;
    let key = runner_key(&served).await?;
    assert!(!key.is_empty());
    let digest_column: String = sqlx::query_scalar("SELECT key_hash FROM runner_service_device")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert_ne!(digest_column, key);
    let sealed: Vec<u8> = sqlx::query_scalar("SELECT key_sealed FROM runner_service_device")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    assert!(
        !sealed
            .windows(key.len())
            .any(|window| window == key.as_bytes()),
        "the sealed column holds the plain credential"
    );
    let _login = computer_login(&served, "desk", "k-desk").await?;
    let listed = get(
        &served.base_url(),
        "/v1/computers",
        &RequestSpec::new("").with_bearer(&phone),
    )
    .await?;
    assert_eq!(listed.status, 200);
    let dump = dump_database(&served.db)?;
    assert_eq!(found_spelling(&dump, key.as_bytes()), None);
    let lines = logs.lines();
    assert!(
        lines.iter().all(|line| !line.contains(&key)),
        "the runner credential reached the server log"
    );
    served.stop().await
}

#[tokio::test]
async fn runner_credential_has_no_account_and_no_admin_power() -> Result<(), String> {
    let served = serve_start("runner_device_denied").await?;
    let phone = make_admin(&served).await?;
    let key = runner_key(&served).await?;
    let base = served.base_url();
    let runner = RequestSpec::new("{}")
        .with_key("k-runner")
        .with_bearer(&key);
    let denied_gets = [
        "/v1/admin/invite",
        "/v1/computers",
        "/v1/streams/after?stream=account",
        "/v1/settings",
    ];
    for path in denied_gets {
        let answer = get(&base, path, &runner).await?;
        assert_eq!(answer.status, 403, "{path}: {}", answer.json);
        assert_eq!(answer.code().as_deref(), Some("device-no-permission"));
    }
    let denied_posts = [
        "/v1/computers/pairing-code",
        "/v1/computers/remove",
        "/v1/account/password",
        "/v1/account/logout-all",
        "/v1/admin/invite/rotate",
        "/v1/streams/append",
    ];
    for path in denied_posts {
        let answer = post(&base, path, &runner).await?;
        assert_eq!(answer.status, 403, "{path}: {}", answer.json);
        assert_eq!(answer.code().as_deref(), Some("device-no-permission"));
    }
    let rows = get(
        &base,
        "/v1/computers",
        &RequestSpec::new("").with_bearer(&phone),
    )
    .await?;
    assert_eq!(rows.status, 200);
    let listed = rows
        .json
        .get("computers")
        .and_then(|value| value.as_array());
    assert_eq!(listed.map(Vec::len), Some(0), "the runner is on the list");
    let mut sse = open_sse(&base, "/v1/streams/notifications", &key, "").await?;
    let progress = sse.recv_json_of("progress").await?;
    assert!(progress.get("streams").is_some(), "{progress}");
    sse.close();
    served.stop().await
}
