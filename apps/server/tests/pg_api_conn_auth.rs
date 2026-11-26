use anybot_app_server as _;
use anybot_app_test as _;
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
use sqlx::Row as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use anybot_app_test::http::{RequestSpec, get, options};
use anybot_app_test::serve::serve_start;

use anybot_server_test::{
    cookie_lines, cookie_of, count_sessions, make_admin, post_cookie_login, set_public_url,
};

#[tokio::test]
async fn t_conn_auth_1() -> Result<(), String> {
    let served = serve_start("t_conn_auth_1").await?;
    let admin = make_admin(&served).await?;
    set_public_url(&served, &admin).await?;
    let https = post_cookie_login(
        &served,
        "alice",
        "k-c1",
        Some("https://s.test"),
        &served.base_url(),
    )
    .await?;
    let local = post_cookie_login(
        &served,
        "alice",
        "k-c2",
        Some(&served.base_url()),
        &served.base_url(),
    )
    .await?;
    let cookies_https = cookie_lines(&https);
    let cookies_local = cookie_lines(&local);
    assert_eq!(cookies_https.len(), 1, "{}", https.json);
    assert_eq!(cookies_local.len(), 1, "{}", local.json);
    for line in &cookies_https {
        assert!(line.contains("HttpOnly"), "{line}");
        assert!(line.contains("SameSite=Strict"), "{line}");
        assert!(line.contains("Path=/"), "{line}");
        assert!(line.contains("Secure"), "{line}");
    }
    for line in &cookies_local {
        assert!(line.contains("HttpOnly"), "{line}");
        assert!(line.contains("SameSite=Strict"), "{line}");
        assert!(line.contains("Path=/"), "{line}");
        assert!(!line.contains("Secure"), "{line}");
    }
    assert_eq!(https.json.get("bearer_key"), None, "{}", https.json);
    assert_eq!(local.json.get("bearer_key"), None, "{}", local.json);
    assert_eq!(https.json.get("session_key"), None, "{}", https.json);
    assert!(cookie_of(&https).is_ok());
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_conn_auth_5() -> Result<(), String> {
    let served = serve_start("t_conn_auth_5").await?;
    let base = served.base_url();
    let preflight = options(
        &base,
        "/v1/account/login",
        &RequestSpec::new("{}").with_header("Origin", "https://other.test"),
    )
    .await?;
    assert!(
        preflight
            .header_values("access-control-allow-origin")
            .is_empty(),
        "{}",
        preflight.json
    );
    let plain = get(
        &base,
        "/v1/server/status",
        &RequestSpec::new("{}").with_header("Origin", "https://other.test"),
    )
    .await?;
    assert!(
        plain
            .header_values("access-control-allow-origin")
            .is_empty()
    );
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn conn_auth_url_credentials_are_rejected_and_not_stored() -> Result<(), String> {
    let served = serve_start("conn_auth_url_credentials").await?;
    let admin = make_admin(&served).await?;
    let answer = get(
        &served.base_url(),
        &format!("/v1/server/status?session_key={admin}"),
        &RequestSpec::new("{}"),
    )
    .await?;
    assert_eq!(answer.status, 401, "{}", answer.json);
    let rows = sqlx::query("SELECT body FROM idempotency")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    for row in rows {
        let body: String = row.try_get("body").map_err(|e| e.to_string())?;
        assert!(!body.contains(&admin), "stored body leaks the bearer");
    }
    let events = sqlx::query("SELECT address, account FROM auth_events")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    for row in events {
        let account: String = row.try_get("account").map_err(|e| e.to_string())?;
        assert!(!account.contains(&admin), "auth event leaks the bearer");
    }
    let sessions = count_sessions(&served).await?;
    assert_eq!(sessions, 1, "the rejected probe must not open a session");
    served.stop().await?;
    Ok(())
}
