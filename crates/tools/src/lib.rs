//! tools · 工具介面＋內建工具（檔案、指令、瀏覽器、螢幕…的對外工具）。
//!
//! 照 `spec/07` §2、`spec/02` 的三個可插拔介面之一。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_domain::TaskStatus;

use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

/// 工具執行結果的占位型別。
#[derive(Debug, Clone)]
pub struct ToolOutcome {
    pub status: TaskStatus,
    /// 工具的實際回傳會變成證據事件（`spec/04` §2）。
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl ToolOutcome {
    #[must_use]
    pub fn new(status: TaskStatus, kind: &str) -> Self {
        ToolOutcome {
            status,
            event: EventEnvelope {
                seq: 0,
                kind: kind.to_owned(),
                at_epoch_seconds: now_epoch_seconds(),
            },
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
