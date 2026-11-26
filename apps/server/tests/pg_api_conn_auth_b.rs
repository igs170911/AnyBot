use serde_json::json;

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

type PwCase = (
    &'static str,
    Option<String>,
    Option<&'static str>,
    bool,
    u16,
);

use anybot_app_test::ClockWalker;
use anybot_app_test::http::ResponseSpec;
use anybot_app_test::http::{RequestSpec, get, post};
use anybot_app_test::serve::serve_start;
use anybot_app_test::sse::open_sse;

use anybot_server_test::{count_sessions, make_admin, set_public_url};

fn pw_body(old: &str, new: &str) -> String {
    json!({"old": old, "new": new}).to_string()
}

fn pw_spec(
    cookie: &str,
    key: &str,
    origin: Option<&str>,
    sec: Option<&str>,
    csrf: bool,
) -> RequestSpec {
    let mut built = RequestSpec::new(&pw_body("alicepassword", "newlongenough"))
        .with_key(key)
        .with_header("Cookie", cookie);
    if csrf {
        built = built.with_header("AnyBot-CSRF", "1");
    }
    if let Some(origin) = origin {
        built = built.with_header("Origin", origin);
    }
    if let Some(sec) = sec {
        built = built.with_header("Sec-Fetch-Site", sec);
    }
    built
}

async fn bearer_pw(
    base: &str,
    clock: &std::sync::Arc<test_harness::FakeClock>,
    bearer: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let mut walker = ClockWalker::start(clock);
    let spec = RequestSpec::new(&pw_body("alicepassword", "newlongenough"))
        .with_key(key)
        .with_bearer(bearer);
    let answer = post(base, "/v1/account/password", &spec).await?;
    walker.stop().await?;
    Ok(answer)
}

fn logout_spec(key: &str, bearer: Option<&str>, origin: Option<&str>) -> RequestSpec {
    let mut built = RequestSpec::new(r#"{"ok":true}"#).with_key(key);
    if let Some(bearer) = bearer {
        built = built.with_bearer(bearer);
    }
    if let Some(origin) = origin {
        built = built.with_header("Origin", origin);
    }
    built
}

fn login_body(credential: Option<&str>, password: &str) -> String {
    let body = match credential {
        Some(credential) => json!({
            "account": "alice",
            "password": password,
            "label": "desk",
            "credential": credential,
        }),
        None => json!({"account": "alice", "password": password, "label": "desk"}),
    };
    body.to_string()
}

async fn cookie_login(
    served: &anybot_app_test::serve::Served,
    key: &str,
) -> Result<String, String> {
    let answer = anybot_server_test::post_cookie_login(
        served,
        "alice",
        key,
        Some(&served.base_url()),
        &served.base_url(),
    )
    .await?;
    anybot_server_test::cookie_of(&answer)
}

#[tokio::test]
async fn t_conn_auth_2() -> Result<(), String> {
    let served = serve_start("t_conn_auth_2").await?;
    let base = served.base_url();
    let admin = make_admin(&served).await?;
    set_public_url(&served, &admin).await?;
    let cookie = cookie_login(&served, "k-co").await?;
    let cases: [PwCase; 4] = [
        (
            "k-pw1",
            Some("https://other.test".to_owned()),
            Some("same-origin"),
            true,
            403,
        ),
        ("k-pw2", None, Some("cross-site"), true, 403),
        ("k-pw3", Some(base.clone()), None, false, 403),
        ("k-pw4", Some(base.clone()), None, true, 200),
    ];
    for (key, origin, sec, csrf, want) in cases {
        let mut walker = ClockWalker::start(&served.clock);
        let spec = pw_spec(&cookie, key, origin.as_deref(), sec, csrf);
        let answer = post(&base, "/v1/account/password", &spec).await?;
        walker.stop().await?;
        assert_eq!(answer.status, want, "{key}: {}", answer.json);
    }
    let mut walker = ClockWalker::start(&served.clock);
    let after = post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(&login_body(Some("bearer"), "alicepassword")).with_key("k-old"),
    )
    .await?;
    walker.stop().await?;
    assert_eq!(
        after.status, 401,
        "old password must be dead: {}",
        after.json
    );
    assert_eq!(
        count_sessions(&served).await?,
        1,
        "cookie session survives login failure"
    );
    served.stop().await?;
    Ok(())
}

async fn sse_bearer_checks(base: &str, admin: &str) -> Result<(), String> {
    let sse_query = open_sse(
        base,
        "/v1/streams/notifications",
        "",
        &format!("session_key={admin}"),
    )
    .await;
    match sse_query {
        Ok(opened) => {
            opened.close();
            return Err("sse with the key in the url must refuse".to_owned());
        }
        Err(refused) => {
            assert!(refused.contains("401"), "sse query auth: {refused}");
        }
    }
    let mut sse_header = open_sse(base, "/v1/streams/notifications", admin, "").await?;
    sse_header.recv_block().await?;
    sse_header.close();
    Ok(())
}

async fn leaks_absent(pool: &anybot_store_server::Pool, admin: &str) -> Result<(), String> {
    let rows = sqlx::query("SELECT body FROM idempotency")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    for row in rows {
        let body: String = row.try_get("body").map_err(|e| e.to_string())?;
        assert!(!body.contains(admin), "stored body leaks the bearer");
    }
    let events = sqlx::query("SELECT address, account FROM auth_events")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    for row in events {
        let account: String = row.try_get("account").map_err(|e| e.to_string())?;
        assert!(!account.contains(admin), "auth event leaks the bearer");
    }
    Ok(())
}

#[tokio::test]
async fn t_conn_auth_3() -> Result<(), String> {
    let served = serve_start("t_conn_auth_3").await?;
    let base = served.base_url();
    let admin = make_admin(&served).await?;
    let in_query = get(
        &base,
        &format!("/v1/admin/invite?session_key={admin}"),
        &RequestSpec::new("{}"),
    )
    .await?;
    assert_eq!(in_query.status, 401, "{}", in_query.json);
    let in_query_post = post(
        &base,
        "/v1/account/logout",
        &logout_spec("k-loq", None, Some(&base)),
    )
    .await?;
    assert_eq!(in_query_post.status, 401, "{}", in_query_post.json);
    let in_header = get(
        &base,
        "/v1/admin/invite",
        &RequestSpec::new("{}").with_bearer(&admin),
    )
    .await?;
    assert_eq!(in_header.status, 200, "{}", in_header.json);
    sse_bearer_checks(&base, &admin).await?;
    let in_header_post = post(
        &base,
        "/v1/account/logout",
        &logout_spec("k-lod", Some(&admin), Some(&base)),
    )
    .await?;
    assert_eq!(in_header_post.status, 200, "{}", in_header_post.json);
    leaks_absent(&served.pool, &admin).await?;
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn t_conn_auth_7() -> Result<(), String> {
    let served = serve_start("t_conn_auth_7").await?;
    let base = served.base_url();
    let admin = make_admin(&served).await?;
    set_public_url(&served, &admin).await?;
    let cookie = cookie_login(&served, "k-co").await?;
    let native_cookie = post(
        &base,
        "/v1/account/password",
        &pw_spec(&cookie, "k-x1", None, None, true),
    )
    .await?;
    assert_eq!(native_cookie.status, 403, "{}", native_cookie.json);
    let bearer_only = bearer_pw(&base, &served.clock, &admin, "k-x2").await?;
    assert_eq!(bearer_only.status, 200, "{}", bearer_only.json);
    let mixed = pw_spec(&cookie, "k-x3", None, None, true).with_bearer(&admin);
    let mixed = post(&base, "/v1/account/password", &mixed).await?;
    assert_eq!(mixed.status, 403, "{}", mixed.json);
    let no_field = post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(&login_body(None, "newlongenough")).with_key("k-x4"),
    )
    .await?;
    assert_eq!(no_field.status, 400, "{}", no_field.json);
    let cookie_no_origin = post(
        &base,
        "/v1/account/login",
        &RequestSpec::new(&login_body(Some("cookie"), "newlongenough"))
            .with_key("k-x5")
            .with_header("AnyBot-CSRF", "1"),
    )
    .await?;
    assert_eq!(cookie_no_origin.status, 403, "{}", cookie_no_origin.json);
    assert_eq!(count_sessions(&served).await?, 1, "single live session");
    served.stop().await?;
    Ok(())
}
