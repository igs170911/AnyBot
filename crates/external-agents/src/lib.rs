//! external-agents · 外部 agent 的轉接（Rust 這一側；Claude Code 的另一側在 adapters/）。
//!
//! 照 `spec/07` §2、`spec/04` §5：外部 agent 的動作同樣要經信任層、證據由程式記。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_foundation::now_epoch_seconds;
use anybot_mcp::McpHttpClient;
use anybot_protocol::EventEnvelope;
use anybot_trust::Evidence;

/// 外部 agent 事件協定的占位：正式協定見 `spec/02` §11。
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub evidence: Evidence,
    /// 外部 agent 的每個動作都要能連到證據（`spec/04` §5）。
    pub inner_event: EventEnvelope,
    pub at_epoch_seconds: i64,
    /// 外部 agent 可能問工具供應者，客戶端型別沿用 mcp。
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
