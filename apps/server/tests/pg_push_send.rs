use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use sqlx as _;
use tokio as _;
use tracing as _;

use std::sync::atomic::Ordering;

use anybot_app_test::db_dump::{dump_database, found_raw_bytes, found_spelling};
use anybot_app_test::serve::serve_start;
use anybot_server_test::make_admin;
use anybot_server_test::push_support::{
    PushMessage, decode_b64url, push_world, server_key, stored_endpoints, subscribe,
};
use anybot_store_server::push_key;
use test_harness::SubscriptionKeys;

fn card_message() -> PushMessage {
    PushMessage::compose(
        "Bot1",
        "push-card-ask",
        &[("command", "npm publish")],
        "bot-1",
        1,
    )
}

#[tokio::test]
async fn t_server_n4_3() -> Result<(), String> {
    let served = serve_start("t_server_n4_3").await?;
    let phone = make_admin(&served).await?;
    let world = push_world(&served).await?;
    let alive = world.register("alive")?;
    let alive_endpoint = world.endpoint("alive");
    let gone_endpoint = world.endpoint("gone");
    let local_endpoint = format!("https://localhost:{}/push/local", world.tls.port);
    for endpoint in [&alive_endpoint, &gone_endpoint, &local_endpoint] {
        let reply = subscribe(&served, &phone, endpoint, &alive).await?;
        assert_eq!(reply.status, 200, "{}", reply.json);
    }
    world.register("local")?;

    let message = card_message();
    let report = world.notifier.notify_account("alice", &message).await?;
    assert_eq!(report.delivered, 1, "{report:?}");
    assert_eq!(report.removed_gone, 1, "{report:?}");
    assert_eq!(report.skipped, 1, "{report:?}");
    assert_eq!(report.failed, 0, "{report:?}");

    let stored = stored_endpoints(&served).await?;
    assert!(stored.contains(&alive_endpoint), "{stored:?}");
    assert!(stored.contains(&local_endpoint), "{stored:?}");
    assert!(!stored.contains(&gone_endpoint), "{stored:?}");

    let records = world.tls.push.records();
    assert_eq!(records.len(), 1, "{records:?}");
    let record = records.first().ok_or("no push record")?;
    assert_eq!(record.subscription_id, "alive");
    assert_eq!(record.ttl.as_deref(), Some("86400"));
    assert_eq!(record.urgency.as_deref(), Some("high"));
    assert_eq!(record.topic.as_deref(), Some("bot-1"));
    let sent: PushMessage = serde_json::from_slice(&record.plaintext).map_err(|e| e.to_string())?;
    assert_eq!(sent, message);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn push_send_does_not_follow_redirect() -> Result<(), String> {
    let served = serve_start("push_send_redirect").await?;
    let phone = make_admin(&served).await?;
    let world = push_world(&served).await?;
    let keys = SubscriptionKeys::generate()?;
    let endpoint = format!(
        "https://{}:{}/redirect",
        anybot_server_test::push_support::PUSH_HOST,
        world.tls.port
    );
    let reply = subscribe(&served, &phone, &endpoint, &keys).await?;
    assert_eq!(reply.status, 200, "{}", reply.json);
    let report = world
        .notifier
        .notify_account("alice", &card_message())
        .await?;
    assert_eq!(report.failed, 1, "{report:?}");
    assert_eq!(report.delivered, 0, "{report:?}");
    assert_eq!(world.tls.landing_hits.load(Ordering::SeqCst), 0);
    assert_eq!(stored_endpoints(&served).await?, vec![endpoint]);
    served.stop().await?;
    Ok(())
}

#[tokio::test]
async fn push_server_key_is_sealed() -> Result<(), String> {
    let served = serve_start("push_server_key_sealed").await?;
    let phone = make_admin(&served).await?;
    let first = server_key(&served, &phone).await?;
    assert_eq!(first.status, 200, "{}", first.json);
    let public = first.field_str("public_key")?;
    assert_eq!(public.len(), 87);
    let again = server_key(&served, &phone).await?;
    assert_eq!(again.field_str("public_key")?, public);

    let keys = served.db.secret_keys()?;
    let stored = push_key::load(&served.pool, &keys)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no stored push key")?;
    assert_eq!(stored.public_key, public);
    let subscriber = SubscriptionKeys::generate()?;
    let made = subscribe(&served, &phone, "https://push.test/sealed", &subscriber).await?;
    assert_eq!(made.status, 200, "{}", made.json);

    let dump = dump_database(&served.db)?;
    assert!(dump.contains(&public), "the public key is stored plainly");
    let private = stored.private.expose();
    assert_eq!(found_spelling(&dump, private.as_bytes()), None);
    let raw = decode_b64url(private)?;
    assert_eq!(found_spelling(&dump, &raw), None);
    assert!(!found_raw_bytes(&dump, &raw));
    assert_eq!(found_spelling(&dump, &subscriber.auth), None);
    assert_eq!(
        found_spelling(&dump, subscriber.auth_b64().as_bytes()),
        None
    );
    served.stop().await?;
    Ok(())
}
