//! domain · 任務、Bot、對話紀錄、核准規則、證據、同步合併規則。
//!
//! 照 `spec/07` §2：純邏輯，不碰網路、檔案、資料庫、非同步執行環境。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// 任務主狀態（`spec/04` §3；細項之後在 task-state 規格補）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Waiting,
    Stuck,
    Done,
    Failed,
}

impl TaskStatus {
    /// 進行中和等你之外的狀態都需要人注意。
    #[must_use]
    pub fn needs_attention(self) -> bool {
        matches!(self, TaskStatus::Stuck | TaskStatus::Failed)
    }
}

use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;

/// 證據清單的項目占位：事件取自 protocol、時間取自 foundation（純型別，不碰 I/O）。
#[derive(Debug, Clone)]
pub struct EvidenceItem {
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
}

impl EvidenceItem {
    #[must_use]
    pub fn new(event: EventEnvelope) -> Self {
        EvidenceItem {
            event,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TaskStatus;

    #[test]
    fn stuck_and_failed_need_attention() {
        assert!(TaskStatus::Stuck.needs_attention());
        assert!(TaskStatus::Failed.needs_attention());
        assert!(!TaskStatus::Running.needs_attention());
    }
}
