//! channels · 聊天頻道轉接：Slack、Discord、Telegram（`spec/09` K42）。
//!
//! 照 `spec/07` §2：發 HTTP 只准發生在這裡。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「發 HTTP 請求」禁用：channels 是准許的套件（spec/07 §3）。
#![allow(clippy::disallowed_methods)]

/// 頻道 API 共用的 HTTP 客戶端型別。
pub type ChannelHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> ChannelHttpClient {
    reqwest::Client::new()
}

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

/// 從頻道收到一則訊息的占位：頻道金鑰用 Secret、訊息進協定事件。
#[derive(Debug)]
pub struct IncomingMessage {
    pub channel_token: Secret,
    pub event: EventEnvelope,
    /// 收到訊息可能開新任務回合，狀態由 domain 定義。
    pub resulting_status: TaskStatus,
}
