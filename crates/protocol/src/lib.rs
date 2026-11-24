//! protocol · 執行事件協定、API 的請求和回應型別（伺服器、執行端、畫面共用）。
//!
//! 照 `spec/07` §2。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_foundation::now_epoch_seconds;
use serde::{Deserialize, Serialize};

/// 執行事件協定的骨架占位：正式協定內容在後續里程碑定義（`spec/02`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub seq: u64,
    pub kind: String,
    /// 事件產生時間（epoch 秒），取自 foundation。
    pub at_epoch_seconds: i64,
}

impl EventEnvelope {
    #[must_use]
    pub fn now(seq: u64, kind: &str) -> Self {
        EventEnvelope {
            seq,
            kind: kind.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
