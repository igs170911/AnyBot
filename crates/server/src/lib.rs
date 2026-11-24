//! server · 伺服器 API（axum）：帳號、信箱、協調、排程、推播、SSE。
//!
//! 照 `spec/07` §3：不准依賴 runner、models、tools、computers（Cargo 就擋住
//! 「伺服器 API 不跑腦」，`spec/02` A1）。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_blob::BlobRef;
use anybot_bus::Notification;
use anybot_channels::ChannelHttpClient;
use anybot_domain::TaskStatus;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_server::Pool;

/// API 占位型別：把資料庫、通知、頻道、任務狀態湊在一起。
#[derive(Debug)]
pub struct AppStateRef {
    pub status: TaskStatus,
}

impl AppStateRef {
    #[must_use]
    pub fn example() -> Self {
        AppStateRef {
            status: TaskStatus::Running,
        }
    }
}

/// 用到的下層型別（骨架階段以型別別名串起來，確保依賴方向正確）。
pub type ServerPool = Pool;
pub type ServerNotification = Notification;
pub type ServerChannelClient = ChannelHttpClient;

/// 伺服器收到協定事件的占位：可以附檔案和時間（骨架把型別湊起來）。
#[derive(Debug)]
pub struct ServerEventBundle {
    pub event: EventEnvelope,
    pub blob: Option<BlobRef>,
    pub at_epoch_seconds: i64,
}

impl ServerEventBundle {
    #[must_use]
    pub fn new(event: EventEnvelope) -> Self {
        ServerEventBundle {
            event,
            blob: None,
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
