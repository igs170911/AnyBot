pub type ModelHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> ModelHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

#[derive(Debug)]
pub struct ModelRequest {
    pub api_key: Secret,
    pub event: EventEnvelope,
    pub resulting_status: TaskStatus,
}
