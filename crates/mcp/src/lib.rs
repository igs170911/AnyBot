pub type McpHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> McpHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::RunEvent;

#[derive(Debug)]
pub struct ConnectorCall {
    pub api_key: Secret,
    pub event: RunEvent,
    pub resulting_status: TaskStatus,
}
