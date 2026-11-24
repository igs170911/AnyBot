//! trust · 信任層：把關、留證據、驗收、產生交付報告。
//!
//! 照 `spec/07` §2、`spec/04` §1–§2。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::EventEnvelope;

/// 證據占位：證據内容由第 2 組規格（`spec/10-behavior/2-trust.md`）定義。
#[derive(Debug, Clone)]
pub struct Evidence {
    pub event: EventEnvelope,
}

/// 金鑰經過信任層時一律用 foundation 的 Secret 型別傳遞。
pub type PassedSecret = Secret;

#[must_use]
pub fn status_needs_attention(status: TaskStatus) -> bool {
    status.needs_attention()
}
