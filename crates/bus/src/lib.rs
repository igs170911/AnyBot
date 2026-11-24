//! bus · 即時通知匯流排：程式內部／NATS（`spec/06` §2.2）。
//!
//! 照 `spec/07` §2。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_foundation::now_epoch_seconds;

/// 通知占位型別：通知只是提示，紀錄在資料庫（`spec/02` §4.1）。
#[derive(Debug, Clone)]
pub struct Notification {
    pub kind: String,
    pub at_epoch_seconds: i64,
}

impl Notification {
    #[must_use]
    pub fn new(kind: &str) -> Self {
        Notification {
            kind: kind.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
