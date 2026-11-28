use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::Row as _;

use anybot_app_test::http::{RequestSpec, ResponseSpec, delete, get, post};
use anybot_app_test::serve::Served;
pub use anybot_channels::web_push::PushMessage;
use anybot_channels::web_push::{PushClient, PushResolver, ResolveFuture, Sender};
use anybot_foundation::Clock;
use anybot_server::push_notifier::WebPushNotifier;
use test_harness::{PushTls, SubscriptionKeys, push_client_builder, push_start_tls};

pub const PUSH_HOST: &str = "push.test";
pub const PUBLIC_ADDRESS: &str = "93.184.216.34";

pub struct PublicResolver;

impl PushResolver for PublicResolver {
    fn resolve<'a>(&'a self, _host: &'a str, _port: u16) -> ResolveFuture<'a> {
        Box::pin(async move {
            let address = PUBLIC_ADDRESS
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?;
            Ok(vec![address])
        })
    }
}

pub struct PushWorld {
    pub tls: PushTls,
    pub notifier: WebPushNotifier,
}

impl PushWorld {
    #[must_use]
    pub fn endpoint(&self, id: &str) -> String {
        format!("https://{PUSH_HOST}:{}/push/{id}", self.tls.port)
    }

    pub fn register(&self, id: &str) -> Result<SubscriptionKeys, String> {
        let keys = SubscriptionKeys::generate()?;
        self.tls.push.register(id, keys.clone());
        Ok(keys)
    }
}

pub async fn push_world(served: &Served) -> Result<PushWorld, String> {
    let tls = push_start_tls(PUSH_HOST).await?;
    let builder = push_client_builder(&tls.cert.pem, PUSH_HOST, tls.port)?;
    let client = PushClient::from_builder(builder).map_err(|e| e.to_string())?;
    let sender = Sender::new(client, Arc::new(PublicResolver));
    let clock: Arc<dyn Clock> = Arc::clone(&served.clock) as Arc<dyn Clock>;
    let notifier = WebPushNotifier::new(
        served.pool.clone(),
        served.db.secret_keys()?,
        clock,
        sender,
        None,
    );
    Ok(PushWorld { tls, notifier })
}

#[must_use]
pub fn subscribe_body(endpoint: &str, keys: &SubscriptionKeys) -> Value {
    json!({
        "endpoint": endpoint,
        "keys": {"p256dh": keys.public_b64(), "auth": keys.auth_b64()},
    })
}

pub async fn subscribe(
    served: &Served,
    bearer: &str,
    endpoint: &str,
    keys: &SubscriptionKeys,
) -> Result<ResponseSpec, String> {
    let body = subscribe_body(endpoint, keys).to_string();
    let spec = RequestSpec::new(&body).with_bearer(bearer);
    post(&served.base_url(), "/v1/push/subscriptions", &spec).await
}

pub async fn unsubscribe(served: &Served, bearer: &str, id: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new("").with_bearer(bearer);
    delete(
        &served.base_url(),
        &format!("/v1/push/subscriptions/{id}"),
        &spec,
    )
    .await
}

pub async fn subscribed_here(served: &Served, bearer: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new("").with_bearer(bearer);
    get(&served.base_url(), "/v1/push/subscriptions/mine", &spec).await
}

pub async fn server_key(served: &Served, bearer: &str) -> Result<ResponseSpec, String> {
    let spec = RequestSpec::new("").with_bearer(bearer);
    get(&served.base_url(), "/v1/push/server-key", &spec).await
}

pub async fn stored_endpoints(served: &Served) -> Result<Vec<String>, String> {
    let rows = sqlx::query("SELECT endpoint FROM push_subscriptions ORDER BY created_at_ms, id")
        .fetch_all(&served.pool)
        .await
        .map_err(|e| e.to_string())?;
    rows.iter()
        .map(|row| row.try_get("endpoint").map_err(|e| e.to_string()))
        .collect()
}

pub fn decode_b64url(text: &str) -> Result<Vec<u8>, String> {
    data_encoding::BASE64URL_NOPAD
        .decode(text.as_bytes())
        .map_err(|e| e.to_string())
}
