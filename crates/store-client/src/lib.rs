//! store-client · 本機資料層（SQLite）：副本、匣子、進行中的回合（`spec/06` S2）。
//!
//! 照 `spec/07` §2：讀寫任意檔案只准發生在這裡（SQLite 檔）。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「讀寫任意檔案」禁用：store-client 是准許的套件（spec/07 §3）。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

/// 本機連線池占位型別。
pub type Pool = sqlx::SqlitePool;

/// 本機副本一筆紀錄的占位。
#[derive(Debug, Clone)]
pub struct ReplicaRecord {
    pub event: EventEnvelope,
    pub status: TaskStatus,
    pub at_epoch_seconds: i64,
}

impl ReplicaRecord {
    #[must_use]
    pub fn new(event: EventEnvelope, status: TaskStatus) -> Self {
        ReplicaRecord {
            event,
            status,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

/// 本機 SQLite 檔存在與否的占位檢查。
#[must_use]
pub fn db_file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
