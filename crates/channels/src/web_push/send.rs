use std::sync::Arc;
use std::time::Duration;

use sad_web_push::request_builder::try_build_request;
use sad_web_push::{
    ContentEncoding, SubscriptionInfo, Urgency, VapidSignatureBuilder, WebPushMessageBuilder,
};

use anybot_foundation::{Clock, Secret};

use super::{PushError, PushMessage, PushResolver, TargetVerdict, VapidKey, check_target};

pub const PUSH_TTL_SECONDS: u32 = 86_400;
pub const PUSH_TIMEOUT: Duration = Duration::from_secs(10);
const VAPID_VALID_SECONDS: u64 = 12 * 60 * 60;

#[derive(Clone)]
pub struct PushClient {
    http: reqwest::Client,
}

impl PushClient {
    pub fn from_builder(builder: reqwest::ClientBuilder) -> Result<PushClient, PushError> {
        let http = builder
            .redirect(reqwest::redirect::Policy::none())
            .timeout(PUSH_TIMEOUT)
            .build()
            .map_err(|source| PushError::Client(source.to_string()))?;
        Ok(PushClient { http })
    }

    pub fn standard() -> Result<PushClient, PushError> {
        PushClient::from_builder(reqwest::Client::builder())
    }
}

#[derive(Debug)]
pub struct Subscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    Delivered,
    Gone,
    Skipped,
    Rejected(u16),
}

pub struct VapidSigning<'a> {
    pub key: &'a VapidKey,
    pub contact: Option<&'a str>,
    pub expires_at_secs: u64,
}

impl<'a> VapidSigning<'a> {
    #[must_use]
    pub fn new(key: &'a VapidKey, contact: Option<&'a str>, clock: &dyn Clock) -> Self {
        let now_secs = clock.now_ms().millis() / 1000;
        VapidSigning {
            key,
            contact,
            expires_at_secs: now_secs.saturating_add(VAPID_VALID_SECONDS),
        }
    }
}

#[derive(Clone)]
pub struct Sender {
    client: PushClient,
    resolver: Arc<dyn PushResolver>,
}

impl Sender {
    #[must_use]
    pub fn new(client: PushClient, resolver: Arc<dyn PushResolver>) -> Sender {
        Sender { client, resolver }
    }

    pub async fn send(
        &self,
        subscription: &Subscription,
        message: &PushMessage,
        vapid: &VapidSigning<'_>,
    ) -> Result<SendOutcome, PushError> {
        let verdict = check_target(&subscription.endpoint, self.resolver.as_ref()).await;
        if verdict != TargetVerdict::Allowed {
            return Ok(SendOutcome::Skipped);
        }
        let request = build_request(subscription, message, vapid)?;
        let response = self
            .client
            .http
            .execute(request)
            .await
            .map_err(|source| PushError::Transport(source.without_url().to_string()))?;
        Ok(outcome_of(response.status().as_u16()))
    }
}

fn outcome_of(status: u16) -> SendOutcome {
    match status {
        200..=299 => SendOutcome::Delivered,
        404 | 410 => SendOutcome::Gone,
        other => SendOutcome::Rejected(other),
    }
}

fn build_request(
    subscription: &Subscription,
    message: &PushMessage,
    vapid: &VapidSigning<'_>,
) -> Result<reqwest::Request, PushError> {
    let info = SubscriptionInfo::new(
        subscription.endpoint.as_str(),
        subscription.p256dh.as_str(),
        subscription.auth.expose(),
    );
    let mut signer = VapidSignatureBuilder::from_base64(vapid.key.scalar().expose(), &info)
        .map_err(|_| PushError::VapidKey)?;
    signer.add_claim("exp", vapid.expires_at_secs);
    if let Some(contact) = vapid.contact {
        signer.add_claim("sub", contact);
    }
    let signature = signer
        .build()
        .map_err(|source| PushError::Build(source.to_string()))?;
    let bytes = message.to_bytes()?;
    let mut builder = WebPushMessageBuilder::new(&info);
    builder.set_payload(ContentEncoding::Aes128Gcm, &bytes);
    builder.set_ttl(PUSH_TTL_SECONDS);
    builder.set_urgency(Urgency::High);
    if let Some(topic) = message.topic() {
        builder.set_topic(topic);
    }
    builder.set_vapid_signature(signature);
    let built = builder
        .build()
        .map_err(|source| PushError::Build(source.to_string()))?;
    let http_request = try_build_request::<Vec<u8>>(built)
        .map_err(|source| PushError::Build(source.to_string()))?;
    reqwest::Request::try_from(http_request).map_err(|source| PushError::Build(source.to_string()))
}
