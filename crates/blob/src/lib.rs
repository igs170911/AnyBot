//! blob · 檔案儲存：本機檔案系統／S3 相容儲存（`spec/06` §2.2）。
//!
//! 照 `spec/07` §2：讀寫任意檔案只准發生在這裡。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「讀寫任意檔案」禁用：blob 是准許的套件（spec/07 §3）。
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use anybot_foundation::now_epoch_seconds;

/// 本機儲存寫入的占位（個人部署）。
///
/// # Errors
///
/// 檔案系統寫入失敗時回傳 `io::Error`。
pub fn write_local(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// 一個儲存物件的占位型別。
#[derive(Debug, Clone)]
pub struct BlobRef {
    pub path: String,
    pub at_epoch_seconds: i64,
}

impl BlobRef {
    #[must_use]
    pub fn new(path: &str) -> Self {
        BlobRef {
            path: path.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
