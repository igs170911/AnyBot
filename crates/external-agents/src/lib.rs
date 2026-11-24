use anybot_foundation::now_epoch_seconds;
use anybot_mcp::McpHttpClient;
use anybot_protocol::EventEnvelope;
use anybot_trust::Evidence;

#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub evidence: Evidence,
    pub inner_event: EventEnvelope,
    pub at_epoch_seconds: i64,
    pub client: Option<McpHttpClient>,
}

impl AgentEvent {
    #[must_use]
    pub fn at_now(evidence: Evidence, inner_event: EventEnvelope) -> Self {
        AgentEvent {
            evidence,
            inner_event,
            at_epoch_seconds: now_epoch_seconds(),
            client: None,
        }
    }
}
