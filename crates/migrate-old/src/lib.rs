//! migrate-old · 舊資料遷移：只讀 OLD 最後一版的格式（`spec/01`、`01a`、`01b`）。
//!
//! 照 `spec/07` §2。M1 骨架：只定套件邊界，沒有功能邏輯。
//! 遷移測資固定在 `tests/fixtures/old-data/`（`spec/05` §2）。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_client::Pool;

/// 遷移報告的占位型別（報告內容見第 5 組規格）。
#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub notes: Vec<String>,
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl MigrationReport {
    #[must_use]
    pub fn start(event: EventEnvelope) -> Self {
        MigrationReport {
            notes: Vec::new(),
            event,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

pub type TargetPool = Pool;

#[must_use]
pub fn migrated_status_placeholder() -> TaskStatus {
    TaskStatus::Done
}
