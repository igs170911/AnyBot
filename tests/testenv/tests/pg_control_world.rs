use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use serde as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use uuid as _;

use std::time::Duration;

use serde_json::{Value, json};

use test_harness::http::{http_get, test_post};
use test_harness::realtime::timeout_real;
use test_harness::server_client::{Call, ServerApi};
use testenv::env::testenv_start;
use testenv::state::EnvPorts;

const DEADLINE: Duration = Duration::from_secs(90);

async fn started() -> Result<EnvPorts, String> {
    let (state, router, listener) = Box::pin(timeout_real(testenv_start(), DEADLINE)).await??;
    let ports = state.ports.clone();
    tokio::spawn(async move {
        let _served = axum::serve(listener, router).await;
    });
    Box::leak(Box::new(state));
    Ok(ports)
}

async fn control_post(ports: &EnvPorts, path: &str, body: &Value) -> Result<(u16, Value), String> {
    ServerApi::new(ports.control_port)?
        .post(path, body, &Call::default())
        .await
}

async fn control_delete(ports: &EnvPorts, path: &str) -> Result<u16, String> {
    ServerApi::new(ports.control_port)?.delete(path).await
}

fn text_at(value: &Value, path: &[&str]) -> Result<String, String> {
    let mut at = value;
    for key in path {
        at = at.get(*key).ok_or(format!("missing {key} in {value}"))?;
    }
    at.as_str()
        .map(str::to_owned)
        .ok_or(format!("not text: {value}"))
}

fn bot_model(world: &Value) -> Result<String, String> {
    let bot = world
        .get("bots")
        .and_then(|bots| bots.get(0))
        .ok_or(format!("no bot in {world}"))?;
    text_at(bot, &["model"])
}

async fn ask_model(ports: &EnvPorts, model: &str) -> Result<String, String> {
    let body = json!({"model": model, "stream": false, "messages": []});
    let (status, text) =
        test_post(ports.model_port, "/v1/chat/completions", &body.to_string()).await?;
    Ok(format!("{status}:{text}"))
}

#[tokio::test]
async fn seed_gives_every_world_its_own_account_bots_and_model_scripts() -> Result<(), String> {
    let ports = Box::pin(started()).await?;
    let (status, first) = control_post(&ports, "/control/seed", &json!({"bots": ["A"]})).await?;
    assert_eq!(status, 200, "{first}");
    let (_status, second) = control_post(&ports, "/control/seed", &json!({"bots": ["B"]})).await?;
    assert_ne!(
        text_at(&first, &["account"])?,
        text_at(&second, &["account"])?
    );
    let model_a = bot_model(&first)?;
    let model_b = bot_model(&second)?;
    assert!(model_a.starts_with("fake-") && model_a != model_b);
    for (model, text) in [(&model_a, "from a"), (&model_b, "from b")] {
        let script =
            json!({"model": model, "steps": [{"ReplyText": {"text": text, "stream": false}}]});
        let (status, _body) = control_post(&ports, "/control/model/script", &script).await?;
        assert_eq!(status, 202);
    }
    assert!(ask_model(&ports, &model_b).await?.contains("from b"));
    assert!(ask_model(&ports, &model_a).await?.contains("from a"));
    for model in [&model_a, &model_b] {
        let (_status, text) = http_get(
            ports.control_port,
            &format!("/control/model/requests?model={model}"),
        )
        .await?;
        let seen: Vec<Value> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        assert_eq!(seen.len(), 1, "{text}");
    }
    Ok(())
}

#[tokio::test]
async fn disabled_account_is_refused_at_login_and_locks_queue_callers() -> Result<(), String> {
    let ports = Box::pin(started()).await?;
    let (_status, world) = control_post(&ports, "/control/seed", &json!({"bots": []})).await?;
    let account = text_at(&world, &["account"])?;
    let password = text_at(&world, &["password"])?;
    let (status, _body) = control_post(
        &ports,
        "/control/account/disable",
        &json!({"account": account}),
    )
    .await?;
    assert_eq!(status, 204);
    let api = ServerApi::new(ports.server_port)?;
    let login =
        json!({"account": account, "password": password, "label": "t", "credential": "bearer"});
    let (status, problem) = api
        .post("/v1/account/login", &login, &Call::default())
        .await?;
    assert_eq!(status, 403, "{problem}");
    assert_eq!(text_at(&problem, &["code"])?, "account-login-disabled");

    let name = json!({"name": "clock"});
    let (status, _body) = control_post(&ports, "/control/lock", &name).await?;
    assert_eq!(status, 200);
    let second = {
        let ports = ports.clone();
        let name = name.clone();
        tokio::spawn(async move { control_post(&ports, "/control/lock", &name).await })
    };
    test_harness::sleep_real(Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "second holder must still be waiting");
    assert_eq!(control_delete(&ports, "/control/lock/clock").await?, 204);
    let got = timeout_real(second, Duration::from_secs(10))
        .await?
        .map_err(|e| e.to_string())??;
    assert_eq!(got.0, 200);
    assert_eq!(control_delete(&ports, "/control/lock/missing").await?, 404);
    Ok(())
}

#[tokio::test]
async fn account_delete_removes_a_seeded_account_once_and_refuses_the_admin() -> Result<(), String>
{
    let ports = Box::pin(started()).await?;
    let (_status, world) = control_post(&ports, "/control/seed", &json!({"bots": ["A"]})).await?;
    let account = json!({"account": text_at(&world, &["account"])?});
    let (status, _body) = control_post(&ports, "/control/account/delete", &account).await?;
    assert_eq!(status, 204);
    let (status, _body) = control_post(&ports, "/control/account/delete", &account).await?;
    assert_eq!(status, 404);
    let admin = json!({"account": "alice"});
    let (status, _body) = control_post(&ports, "/control/account/delete", &admin).await?;
    assert_eq!(status, 500);
    Ok(())
}
