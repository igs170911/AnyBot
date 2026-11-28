mod payload;
mod send;
mod target;
mod vapid;

#[cfg(test)]
mod payload_tests;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod vapid_tests;

pub use payload::{
    BADGE_MAX, BODY_MAX, DEFAULT_TITLE, PAYLOAD_MAX, PushMessage, QUESTION_MAX, TAG_MAX, TITLE_MAX,
    TOPIC_MAX,
};
pub use send::{
    PUSH_TIMEOUT, PUSH_TTL_SECONDS, PushClient, SendOutcome, Sender, Subscription, VapidSigning,
};
pub use target::{
    ENDPOINT_MAX, PushResolver, ResolveFuture, SystemResolver, TargetVerdict, check_target,
    endpoint_shape_ok, is_public_ip,
};
pub use vapid::VapidKey;

#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("push payload is too large")]
    PayloadTooLarge,
    #[error("push payload could not be serialized: {0}")]
    Serialize(String),
    #[error("vapid key is invalid")]
    VapidKey,
    #[error("random source failed: {0}")]
    Random(String),
    #[error("push message could not be built: {0}")]
    Build(String),
    #[error("push request failed: {0}")]
    Transport(String),
    #[error("push client could not be built: {0}")]
    Client(String),
}
