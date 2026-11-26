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
use anybot_domain::version_release::ReleaseTable;
use anybot_server_test::releases;

const DAY_MS: i64 = 86_400_000;
const BASE_MS: i64 = 1_700_000_000_000;
const OLD_CODE: &str = "install-version-old";

const ROUTES: [(&str, &str); 13] = [
    ("post", "/v1/account/login"),
    ("post", "/v1/account/logout"),
    ("post", "/v1/account/logout-all"),
    ("post", "/v1/account/password"),
    ("post", "/v1/account/register"),
    ("post", "/v1/admin/accounts/alice/disable"),
    ("post", "/v1/admin/accounts/alice/enable"),
    ("post", "/v1/admin/accounts/alice/reset-password"),
    ("get", "/v1/admin/invite"),
    ("post", "/v1/admin/invite/rotate"),
    ("post", "/v1/admin/public-url"),
    ("get", "/v1/health"),
    ("get", "/v1/server/status"),
];

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
    let days = u64::try_from(210 * DAY_MS).map_err(|e| e.to_string())?;
    let _advanced = served.clock.advance(std::time::Duration::from_millis(days));
    Ok(())
}

fn has_server_time(answer: &ResponseSpec) -> bool {
    !answer.header_values("AnyBot-Server-Time").is_empty()
}

fn code_of(answer: &ResponseSpec) -> String {
    answer
        .json
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

#[tokio::test]
async fn every_route_without_version_returns_one_code() -> Result<(), String> {
    let served = serve_start_with("version_routes", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    for (method, path) in ROUTES {
        let spec = route_spec(method, None);
        let answer = dispatch(&base, method, path, &spec).await?;
        let stamp = has_server_time(&answer);
        assert!(stamp, "no server time on {path}: {:?}", answer.headers);
        if path.starts_with("/v1/health") || path.starts_with("/v1/server/status") {
            assert_eq!(answer.status, 200, "exempt {path}: {}", answer.json);
            continue;
        }
        if path == "/v1/admin/invite" {
            assert_eq!(answer.status, 401, "invite reads: {}", answer.json);
            continue;
        }
        assert_eq!(
            answer.status, 403,
            "version gate on {path}: {}",
            answer.json
        );
        assert_eq!(code_of(&answer), OLD_CODE, "{path}: {}", answer.json);
    }
    served.stop().await
}

#[tokio::test]
async fn every_response_carries_server_time() -> Result<(), String> {
    let served = serve_start_with("version_time", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    for (method, path) in ROUTES {
        let spec = route_spec(method, Some("0.1.0"));
        let answer = dispatch(&base, method, path, &spec).await?;
        assert!(
            has_server_time(&answer),
            "no server time on {path}: {:?}",
            answer.headers
        );
    }
    served.stop().await
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

#[tokio::test]
async fn newer_and_replaced_logins_accepted() -> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS + 30 * DAY_MS)]);
    let served = serve_start_with("n25_6_logins", &table).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let newer = bearer_login(
        &served,
        &base,
        "alice",
        "alicepassword",
        "k-newer",
        Some("0.3.0"),
    )
    .await?;
    assert_eq!(newer.status, 200, "newer client: {}", newer.json);
    let replaced = bearer_login(
        &served,
        &base,
        "bob",
        "bobpassword",
        "k-replaced",
        Some("0.1.0"),
    )
    .await?;
    assert_eq!(replaced.status, 200, "replaced client: {}", replaced.json);
    let sessions = anybot_server_test::count_sessions(&served).await?;
    assert_eq!(sessions, 2, "two logins accepted");
    served.stop().await
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

#[tokio::test]
async fn version_gate_keeps_health_and_status_open() -> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS)]);
    let served = serve_start_with("version_exempt", &table).await?;
    let base = served.base_url();
    walk_past_release(&served)?;
    let health = get(&base, "/v1/health", &RequestSpec::new("")).await?;
    assert_eq!(health.status, 200, "health open: {}", health.json);
    assert!(has_server_time(&health), "health stamp");
    let status = get(&base, "/v1/server/status", &RequestSpec::new("")).await?;
    assert_eq!(status.status, 200, "status open: {}", status.json);
    assert!(has_server_time(&status), "status stamp");
    served.stop().await
}

#[tokio::test]
async fn logged_in_cookie_write_blocked_when_too_old() -> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS)]);
    let served = serve_start_with("version_write", &table).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let origin = format!("{base}/");
    let cookie =
        anybot_server_test::post_cookie_login(&served, "alice", "k-login", Some(&origin), &base)
            .await?;
    let lines = anybot_server_test::cookie_lines(&cookie);
    let line = lines
        .first()
        .ok_or("cookie login set no cookie".to_owned())?
        .clone();
    walk_past_release(&served)?;
    let spec = RequestSpec::new("{\"ok\":true}")
        .with_header("Cookie", &line)
        .with_header("AnyBot-CSRF", "1")
        .with_header("AnyBot-Client-Version", "0.1.0");
    let answer = post(&base, "/v1/account/logout", &spec).await?;
    assert_eq!(answer.status, 403, "old write: {}", answer.json);
    assert_eq!(code_of(&answer), OLD_CODE, "old write code");
    served.stop().await
}
