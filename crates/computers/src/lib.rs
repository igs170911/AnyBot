//! computers · 電腦介面＋實作：local（使用者的電腦）、docker、kubernetes、第三方轉接。
//!
//! 照 `spec/07` §2：執行外部程式、發 HTTP、讀寫任意檔案只准發生在這裡，
//! 而且執行外部程式要經過 trust 給的通行證（`spec/05` §2）。
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的三類禁用：computers 是准許的套件（spec/07 §3）。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

/// 在 Bot 的電腦上跑指令的占位：正式實作要帶 trust 的通行證。
#[must_use]
pub fn command_shell(program: &str) -> std::process::Command {
    std::process::Command::new(program)
}

/// 電腦上讀檔的占位（工作資料夾規則之後在 trust 定）。
///
/// # Errors
///
/// 檔案系統讀取失敗時回傳 `io::Error`。
pub fn read_file_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

use anybot_domain::TaskStatus;
use anybot_foundation::{Secret, now_epoch_seconds};
use anybot_protocol::EventEnvelope;

/// 在電腦上做一個動作的占位：把關後才產生，帶事件、時間、金鑰遮罩。
#[derive(Debug)]
pub struct ComputerAction {
    pub event: EventEnvelope,
    pub at_epoch_seconds: i64,
    pub secrets_used: Vec<Secret>,
    pub resulting_status: TaskStatus,
    /// 要連到假網站時用 computers 自己的 HTTP 客戶端。
    pub http: Option<reqwest::Client>,
}

impl ComputerAction {
    #[must_use]
    pub fn at_now(event: EventEnvelope, status: TaskStatus) -> Self {
        ComputerAction {
            event,
            at_epoch_seconds: now_epoch_seconds(),
            secrets_used: Vec::new(),
            resulting_status: status,
            http: None,
        }
    }
}
