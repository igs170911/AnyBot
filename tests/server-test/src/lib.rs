pub mod computers_support;
pub mod mailbox_support;

use std::collections::BTreeMap;

use serde_json::json;

use sqlx::Row as _;

use anybot_app_test::ClockWalker;
use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::Served;
use anybot_domain::version_release::{ReleaseRow, ReleaseTable};

#[must_use]
pub fn login_cookie_body(account: &str, password: &str) -> String {
    json!({
        "account": account,
        "password": password,
        "label": "browser",
        "credential": "cookie",
    })
    .to_string()
}

#[must_use]
pub fn register_body(account: &str, password: &str, invite: &str) -> String {
    json!({"account": account, "password": password, "invite": invite}).to_string()
}

pub async fn make_admin(served: &Served) -> Result<String, String> {
    let invite = served
        .started
        .new_invite
        .clone()
        .ok_or("fresh boot must draw an invite")?;
    let created = post(
        &served.base_url(),
        "/v1/account/register",
        &RequestSpec::new(&register_body("alice", "alicepassword", &invite)).with_key("k-reg"),
    )
    .await?;
    assert_eq!(created.status, 201, "{}", created.json);
    let mut walker = ClockWalker::start(&served.clock);
    let logged = post(
        &served.base_url(),
        "/v1/account/login",
        &RequestSpec::new(
            &json!({
                "account": "alice",
                "password": "alicepassword",
                "label": "desk",
                "credential": "bearer",
            })
            .to_string(),
        )
        .with_key("k-login"),
    )
    .await?;
    walker.stop().await?;
    assert_eq!(logged.status, 200, "{}", logged.json);
    logged.bearer()
}

pub async fn set_public_url(served: &Served, bearer: &str) -> Result<(), String> {
    let mut walker = ClockWalker::start(&served.clock);
    let changed = post(
        &served.base_url(),
        "/v1/admin/public-url",
        &RequestSpec::new("{\"public_url\":\"https://s.test\"}")
            .with_bearer(bearer)
            .with_key("k-url"),
    )
    .await?;
    walker.stop().await?;
    assert_eq!(changed.status, 200, "{}", changed.json);
    Ok(())
}

#[must_use]
pub fn cookie_lines(answer: &ResponseSpec) -> Vec<String> {
    answer.header_values("set-cookie")
}

pub fn cookie_of(answer: &ResponseSpec) -> Result<String, String> {
    for line in cookie_lines(answer) {
        let Some((name, value)) = line
            .split(';')
            .next()
            .and_then(|first| first.split_once('='))
        else {
            continue;
        };
        if name.trim() == "anybot_session" && !value.trim().is_empty() {
            return Ok(format!("anybot_session={value}"));
        }
    }
    Err(format!("no session cookie in {:?}", answer.json))
}

pub async fn post_cookie_login(
    served: &Served,
    account: &str,
    key: &str,
    origin: Option<&str>,
    base: &str,
) -> Result<ResponseSpec, String> {
    let mut walker = ClockWalker::start(&served.clock);
    let mut spec = RequestSpec::new(&login_cookie_body(account, "alicepassword"))
        .with_key(key)
        .with_header("AnyBot-CSRF", "1");
    if let Some(origin) = origin {
        spec = spec.with_header("Origin", origin);
    }
    let answer = post(base, "/v1/account/login", &spec).await?;
    walker.stop().await?;
    assert_eq!(answer.status, 200, "cookie login: {}", answer.json);
    Ok(answer)
}

pub async fn count_sessions(served: &Served) -> Result<i64, String> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM sessions")
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    row.try_get("n").map_err(|e| e.to_string())
}

pub async fn table_counts(served: &Served) -> Result<BTreeMap<String, i64>, String> {
    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
    )
    .fetch_all(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut counts = BTreeMap::new();
    for row in rows {
        let name: String = row.try_get("table_name").map_err(|e| e.to_string())?;
        if name == "sessions" {
            continue;
        }
        let count = sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) AS n FROM {name}"
        )))
        .fetch_one(&served.pool)
        .await
        .map_err(|e| e.to_string())?
        .try_get("n")
        .map_err(|e| e.to_string())?;
        counts.insert(name, count);
    }
    Ok(counts)
}

#[must_use]
pub fn releases(rows: &[(&str, i64)]) -> ReleaseTable {
    ReleaseTable {
        rows: rows
            .iter()
            .map(|(version, at_ms)| ReleaseRow {
                version: (*version).to_owned(),
                at_ms: *at_ms,
            })
            .collect(),
    }
}
