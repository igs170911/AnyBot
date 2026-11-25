pub use anybot_foundation::Secret;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;
use anybot_store_client as store_client;

pub type SyncHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> SyncHttpClient {
    reqwest::Client::new()
}

#[derive(Debug)]
pub struct BoxedRecords {
    pub pool_ref: Option<store_client::Pool>,
    pub events: Vec<RunEvent>,
    pub packed_at: TimestampMs,
    pub server_token: Secret,
}

impl BoxedRecords {
    #[must_use]
    pub fn pack(events: Vec<RunEvent>, server_token: Secret, clock: &dyn Clock) -> Self {
        BoxedRecords {
            pool_ref: None,
            events,
            packed_at: clock.now_ms(),
            server_token,
        }
    }
}
