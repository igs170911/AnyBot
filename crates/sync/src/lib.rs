pub use anybot_foundation::Secret;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_client as store_client;

pub type SyncHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> SyncHttpClient {
    reqwest::Client::new()
}

#[derive(Debug, Clone)]
pub struct BoxedRecords {
    pub pool_ref: Option<store_client::Pool>,
    pub events: Vec<EventEnvelope>,
    pub packed_at_epoch_seconds: i64,
    pub server_token: Secret,
}

impl BoxedRecords {
    #[must_use]
    pub fn pack(events: Vec<EventEnvelope>, server_token: Secret) -> Self {
        BoxedRecords {
            pool_ref: None,
            events,
            packed_at_epoch_seconds: now_epoch_seconds(),
            server_token,
        }
    }
}
