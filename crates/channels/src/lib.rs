pub type ChannelHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> ChannelHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

#[derive(Debug)]
pub struct IncomingMessage {
    pub channel_token: Secret,
    pub event: EventEnvelope,
    pub resulting_status: TaskStatus,
}
