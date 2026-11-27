pub use anybot_foundation::Secret;
use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;
use anybot_store_client as store_client;

#[cfg(test)]
use proptest as _;
pub mod after_merge;
pub mod heartbeat_idle;
pub mod heartbeat_keepalive;
pub mod outbox;
pub mod reconnect_connect;
pub mod reconnect_direct;
pub mod reconnect_http;
pub mod reconnect_pacing;
pub mod reconnect_random;
pub mod reconnect_status;
pub mod sse_hint;
pub mod sse_parse;
pub mod sse_session;

pub use after_merge::{Replica, StreamDelta, StreamItem, delta_from};
pub use outbox::{RETRY_CAP_SECONDS, retry_delay_secs, retry_schedule};

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

#[cfg(test)]
mod after_merge_tests;
#[cfg(test)]
mod outbox_tests;
#[cfg(test)]
mod sync_id_tests;
