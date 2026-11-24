//! runner · 執行端：Bot 的回合（問模型 → 經過信任層 → 工具）。
//!
//! 照 `spec/07` §2：可以用大部分下層套件，但不准 store-server、server。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_trust::Evidence;

use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

/// 一個回合的占位型別：問模型 → 把關 → 工具，細節在後續里程碑。
#[derive(Debug, Clone)]
pub struct Turn {
    pub evidence: Vec<Evidence>,
    pub events: Vec<EventEnvelope>,
    pub status: TaskStatus,
    pub started_at_epoch_seconds: i64,
}

impl Turn {
    #[must_use]
    pub fn new(status: TaskStatus) -> Self {
        Turn {
            evidence: Vec::new(),
            events: Vec::new(),
            status,
            started_at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
