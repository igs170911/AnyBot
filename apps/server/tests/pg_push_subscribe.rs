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
use test_harness::SubscriptionKeys;
use tokio as _;
use tracing as _;

use std::time::Duration;

use anybot_app_test::serve::serve_start;
use anybot_server_test::computers_support::computer_login;
use anybot_server_test::make_admin;
use anybot_server_test::push_support::{
    push_world, server_key, stored_endpoints, subscribe, subscribed_here, unsubscribe,
};

const ENDPOINT_MAX: usize = 2048;

fn padded(prefix: &str, total: usize) -> String {
    let fill = total.saturating_sub(prefix.chars().count());
    format!("{prefix}{}", "a".repeat(fill))
}

#[tokio::test]
async fn t_server_n4_2() -> Result<(), String> {
    let served = serve_start("t_server_n4_2").await?;
    let phone = make_admin(&served).await?;
    let world = push_world(&served).await?;
    let keys = SubscriptionKeys::generate()?;

    let plain_http = subscribe(&served, &phone, "http://push.test/abc", &keys).await?;
    assert_eq!(plain_http.status, 400, "{}", plain_http.json);
    assert_eq!(plain_http.code().as_deref(), Some("push-endpoint-invalid"));
    let too_long = padded("https://push.test/", ENDPOINT_MAX + 1);
    let refused = subscribe(&served, &phone, &too_long, &keys).await?;
    assert_eq!(refused.status, 400, "{}", refused.json);
    assert_eq!(refused.code().as_deref(), Some("push-endpoint-invalid"));
    assert!(stored_endpoints(&served).await?.is_empty());

    let same = world.endpoint("same");
    let first = subscribe(&served, &phone, &same, &keys).await?;
    let second = subscribe(&served, &phone, &same, &keys).await?;
    assert_eq!(first.status, 200, "{}", first.json);
    assert_eq!(second.status, 200, "{}", second.json);
    assert_eq!(stored_endpoints(&served).await?, vec![same.clone()]);

    for n in 1..16 {
        let _woken = served.clock.advance(Duration::from_secs(1));
        let reply = subscribe(&served, &phone, &world.endpoint(&format!("e{n}")), &keys).await?;
        assert_eq!(reply.status, 200, "{}", reply.json);
    }
    let before = stored_endpoints(&served).await?;
    assert_eq!(before.len(), 16);
    assert_eq!(before.first(), Some(&same));

    let _woken = served.clock.advance(Duration::from_secs(1));
    let seventeenth = world.endpoint("e16");
    let reply = subscribe(&served, &phone, &seventeenth, &keys).await?;
    assert_eq!(reply.status, 200, "{}", reply.json);
    let after = stored_endpoints(&served).await?;
    assert_eq!(after.len(), 16);
    assert!(
        !after.contains(&same),
        "oldest subscription must be dropped"
    );
    assert_eq!(after.last(), Some(&seventeenth));
    assert_eq!(after.first(), Some(&world.endpoint("e1")));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn subscribe_accepts_exactly_the_longest_endpoint() -> Result<(), String> {
    let served = serve_start("push_subscribe_longest").await?;
    let phone = make_admin(&served).await?;
    let keys = SubscriptionKeys::generate()?;
    let longest = padded("https://push.test/", ENDPOINT_MAX);
    let reply = subscribe(&served, &phone, &longest, &keys).await?;
    assert_eq!(reply.status, 200, "{}", reply.json);
    assert_eq!(stored_endpoints(&served).await?, vec![longest]);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn subscription_keys_with_wrong_length_are_refused() -> Result<(), String> {
    let served = serve_start("push_subscribe_bad_keys").await?;
    let phone = make_admin(&served).await?;
    let keys = SubscriptionKeys::generate()?;
    let body = serde_json::json!({
        "endpoint": "https://push.test/abc",
        "keys": {"p256dh": "short", "auth": keys.auth_b64()},
    })
    .to_string();
    let spec = anybot_app_test::http::RequestSpec::new(&body).with_bearer(&phone);
    let reply =
        anybot_app_test::http::post(&served.base_url(), "/v1/push/subscriptions", &spec).await?;
    assert_eq!(reply.status, 400, "{}", reply.json);
    assert!(stored_endpoints(&served).await?.is_empty());
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn mine_follows_the_subscription_of_this_device() -> Result<(), String> {
    let served = serve_start("push_subscribe_mine").await?;
    let phone = make_admin(&served).await?;
    let keys = SubscriptionKeys::generate()?;
    let before = subscribed_here(&served, &phone).await?;
    assert_eq!(
        before.json.get("subscribed"),
        Some(&serde_json::Value::Bool(false)),
        "{}",
        before.json
    );
    let made = subscribe(&served, &phone, "https://push.test/mine", &keys).await?;
    let id = made.field_str("id")?;
    let during = subscribed_here(&served, &phone).await?;
    assert_eq!(
        during.json.get("subscribed"),
        Some(&serde_json::Value::Bool(true)),
        "{}",
        during.json
    );
    let removed = unsubscribe(&served, &phone, &id).await?;
    assert_eq!(removed.status, 200, "{}", removed.json);
    let after = subscribed_here(&served, &phone).await?;
    assert_eq!(
        after.json.get("subscribed"),
        Some(&serde_json::Value::Bool(false)),
        "{}",
        after.json
    );
    let again = unsubscribe(&served, &phone, &id).await?;
    assert_eq!(again.status, 404, "{}", again.json);
    assert_eq!(again.code().as_deref(), Some("push-subscription-not-found"));
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn computer_credentials_cannot_subscribe() -> Result<(), String> {
    let served = serve_start("push_subscribe_computer").await?;
    make_admin(&served).await?;
    let computer = computer_login(&served, "A", "k-a").await?;
    let keys = SubscriptionKeys::generate()?;
    let reply = subscribe(&served, &computer.bearer, "https://push.test/c", &keys).await?;
    assert_eq!(reply.status, 403, "{}", reply.json);
    let key = server_key(&served, &computer.bearer).await?;
    assert_eq!(key.status, 403, "{}", key.json);
    served.stop().await?;
    Ok(())
}
