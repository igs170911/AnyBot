//! models · 模型供應商介面＋各家實作（OpenAI 相容、Anthropic、Gemini…）。
//!
//! 照 `spec/07` §2；發 HTTP 只准發生在這裡（`spec/07` §3）。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「發 HTTP 請求」禁用：models 是准許的套件之一。
#![allow(clippy::disallowed_methods)]

/// 所有模型供應商共用的 HTTP 客戶端型別（reqwest＋rustls，`spec/06` §2.2）。
pub type ModelHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> ModelHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

/// 模型請求的占位：金鑰用 foundation 的 Secret、事件協定用 protocol。
#[derive(Debug)]
pub struct ModelRequest {
    pub api_key: Secret,
    pub event: EventEnvelope,
    /// 模型回覆之後任務的狀態由 domain 定義。
    pub resulting_status: TaskStatus,
}
