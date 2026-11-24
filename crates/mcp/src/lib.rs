//! mcp · MCP 客戶端（接外部工具）＋ MCP 伺服器（把 `AnyBot` 的工具提供給外部 agent）。
//!
//! 照 `spec/07` §2；發 HTTP 只准發生在這裡（`spec/07` §3）。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「發 HTTP 請求」禁用：mcp 是准許的套件之一。
#![allow(clippy::disallowed_methods)]

/// 網址型 MCP 連接器共用的 HTTP 客戶端型別。
pub type McpHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> McpHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

/// 連接器工具的占位：金鑰用 Secret（不能印）、結果進任務狀態。
#[derive(Debug)]
pub struct ConnectorCall {
    pub api_key: Secret,
    pub event: EventEnvelope,
    pub resulting_status: TaskStatus,
}
