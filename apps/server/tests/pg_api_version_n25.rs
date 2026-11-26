use serde_json::json;

use anybot_app_server as _;
use anybot_app_test as _;
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

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::serve::serve_start_with;
use anybot_server_test::{releases, table_counts};

const DAY_MS: i64 = 86_400_000;
const BASE_MS: i64 = 1_700_000_000_000;
const OLD_CODE: &str = "install-version-old";

fn route_spec(method: &str, version: Option<&str>) -> RequestSpec {
    let body = if method == "post" {
        "{\"ok\":true}"
    } else {
        ""
    };
    let built = RequestSpec::new(body);
    match version {
        Some(value) => built.with_header("AnyBot-Client-Version", value),
        None => built.without_version(),
    }
}

fn walk_past_release(served: &anybot_app_test::serve::Served) -> Result<(), String> {
    let days = u64::try_from(220 * DAY_MS).map_err(|e| e.to_string())?;
    let _advanced = served.clock.advance(std::time::Duration::from_millis(days));
    Ok(())
}

fn code_of(answer: &ResponseSpec) -> String {
    answer
        .json
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

async fn dispatch(
    base: &str,
    method: &str,
    path: &str,
    spec: &RequestSpec,
) -> Result<ResponseSpec, String> {
    match method {
        "get" => get(base, path, spec).await,
        _ => post(base, path, spec).await,
    }
}

async fn bearer_login(
    served: &anybot_app_test::serve::Served,
    base: &str,
    account: &str,
    password: &str,
    key: &str,
    version: Option<&str>,
) -> Result<ResponseSpec, String> {
    let body = json!({
        "account": account,
        "password": password,
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let mut spec = RequestSpec::new(&body).with_key(key);
    if let Some(value) = version {
        spec = spec.with_header("AnyBot-Client-Version", value);
    }
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(base, "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer
}

#[tokio::test]
async fn t_install_n25_5() -> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS), ("0.3.0", BASE_MS + 30 * DAY_MS)]);
    let served = serve_start_with("n25_5_blocked", &table).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let old = bearer_login(
        &served,
        &base,
        "alice",
        "alicepassword",
        "k-n25-old",
        Some("0.1.0"),
    )
    .await?;
    assert_eq!(old.status, 200, "old login: {}", old.json);
    let plain = bearer_login(
        &served,
        &base,
        "bob",
        "bobpassword",
        "k-n25-plain",
        Some(anybot_app_test::http::TEST_CLIENT_VERSION),
    )
    .await?;
    assert_eq!(plain.status, 200, "plain login: {}", plain.json);
    walk_past_release(&served)?;
    let seed = json!({
        "stream": "account",
        "items": [{
            "device_id": "dev-n25",
            "kind": "message",
            "content": {"text": "blocked"},
            "created_at_ms": 1_700_000_000_100_i64,
            "offline_sent": false,
        }],
    })
    .to_string();
    let old_token = old.bearer()?;
    let plain_token = plain.bearer()?;
    for token in &[&old_token, &plain_token] {
        blocked_route_set(&base, token).await?;
    }

    let body = RequestSpec::new(&seed).with_key("k-n25-send");
    let plain_send = post(&base, "/v1/streams/append", &body.without_version()).await?;
    assert_eq!(plain_send.status, 403, "plain append: {}", plain_send.json);
    assert_eq!(code_of(&plain_send), OLD_CODE);
    assert_eq!(
        anybot_server_test::count_sessions(&served).await?,
        2,
        "no new sessions"
    );
    let counts = table_counts(&served).await?;
    assert_eq!(
        counts.get("records").copied().unwrap_or(-1),
        0,
        "nothing stored"
    );
    served.stop().await
}

async fn blocked_route_set(base: &str, token: &str) -> Result<(), String> {
    blocked_route(base, "post", "/v1/account/login", token)
        .await
        .map_err(|e| format!("LOGIN {e}"))?;
    blocked_route(base, "post", "/v1/streams/append", token).await?;
    blocked_route(base, "post", "/v1/tasks/nope/steps", token).await?;
    blocked_route(base, "get", "/v1/streams/notifications", token).await?;
    let spec = route_spec("post", Some("0.2.0")).with_bearer(token);
    let health = dispatch(base, "get", "/v1/health", &spec).await?;
    assert_eq!(health.status, 200, "health: {}", health.json);
    let status = dispatch(base, "get", "/v1/server/status", &spec).await?;
    assert_eq!(status.status, 200, "status: {}", status.json);
    Ok(())
}

async fn blocked_route(base: &str, method: &str, path: &str, token: &str) -> Result<(), String> {
    let spec = route_spec(method, Some("0.2.0")).with_bearer(token);
    let answer = dispatch(base, method, path, &spec).await?;
    assert_eq!(
        answer.status,
        403,
        "{path} v{} bearer: {}",
        spec.dump(),
        answer.json,
    );
    assert_eq!(code_of(&answer), OLD_CODE, "{path} code");
    Ok(())
}

#[tokio::test]
async fn two_clients_blocked_with_same_code() -> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS)]);
    let served = serve_start_with("n25_5_two", &table).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let first = bearer_login(
        &served,
        &base,
        "alice",
        "alicepassword",
        "k-live",
        Some("0.2.0"),
    )
    .await?;
    assert_eq!(first.status, 200, "seed login: {}", first.json);
    walk_past_release(&served)?;
    let old = bearer_login(
        &served,
        &base,
        "alice",
        "alicepassword",
        "k-old",
        Some("0.1.0"),
    )
    .await?;
    let no_version = bearer_login(&served, &base, "bob", "bobpassword", "k-bare", None).await?;
    assert_eq!(old.status, 403, "old client login: {}", old.json);
    assert_eq!(
        no_version.status, 403,
        "versionless login: {}",
        no_version.json
    );
    assert_eq!(code_of(&old), OLD_CODE, "old login code");
    assert_eq!(code_of(&no_version), OLD_CODE, "versionless code");
    let report = password_change_step(&base, &first, "0.1.0").await?;
    assert_eq!(report.status, 403, "runner report: {}", report.json);
    assert_eq!(code_of(&report), OLD_CODE, "report code");
    let sessions = anybot_server_test::count_sessions(&served).await?;
    assert_eq!(sessions, 1, "blocked writes add no sessions");
    served.stop().await
}

async fn password_change_step(
    base: &str,
    login: &ResponseSpec,
    version: &str,
) -> Result<ResponseSpec, String> {
    let origin = format!("{base}/");
    let token = login.bearer()?;
    let spec = RequestSpec::new(&pw_body())
        .with_key("k-report")
        .with_header("AnyBot-Client-Version", version)
        .with_header("Origin", &origin)
        .with_header("AnyBot-CSRF", "1")
        .with_bearer(&token);
    post(base, "/v1/account/password", &spec).await
}

fn pw_body() -> String {
    json!({"old": "alicepassword", "new": "newlongenough"}).to_string()
}
