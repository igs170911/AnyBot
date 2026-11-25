use anybot_foundation::{Clock, TimestampMs};
use anybot_mcp::McpHttpClient;
use anybot_protocol::RunEvent;
use anybot_trust::Evidence;

#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub evidence: Evidence,
    pub inner_event: RunEvent,
    pub at: TimestampMs,
    pub client: Option<McpHttpClient>,
}

impl AgentEvent {
    #[must_use]
    pub fn at_now(evidence: Evidence, inner_event: RunEvent, clock: &dyn Clock) -> Self {
        AgentEvent {
            evidence,
            inner_event,
            at: clock.now_ms(),
            client: None,
        }
    }
}
