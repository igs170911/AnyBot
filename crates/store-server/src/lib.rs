//! store-server · 伺服器資料層（PostgreSQL，`spec/06` S2）。
//!
//! 照 `spec/07` §2。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_domain::TaskStatus;
use anybot_foundation::FoundationError;
use anybot_protocol::EventEnvelope;

/// 伺服器連線池占位型別；SQL 編譯時檢查（sqlx query macros）在後續里程碑接上。
pub type Pool = sqlx::PgPool;

/// 一筆伺服器紀錄的占位：寫入時帶協定事件和任務狀態（每個步驟完成寫一筆，`spec/06` S6）。
#[derive(Debug, Clone)]
pub struct ServerRecord {
    pub event: EventEnvelope,
    pub status: TaskStatus,
}

/// 骨架階段沒有連線函式；設定缺少的錯誤沿用 foundation 的型別。
#[must_use]
pub fn placeholder_missing_bootstrap() -> FoundationError {
    FoundationError::ConfigMissing("--bootstrap")
}
