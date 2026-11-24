//! foundation · 最底層：ID、時間、錯誤、Secret 型別、啟動設定。
//!
//! 照 `spec/07` §3（2026-11-24 修正）：讀環境變數任何地方都不准；
//! foundation 的檔案豁免只准讀 `--bootstrap` 指定的那個啟動檔。
//!
//! M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的禁用：只准用在底下 read_bootstrap_file 讀啟動檔這一處
// （「讀環境變數」即使在 allow 裡出現文字，depcheck 也會擋）。
#![allow(clippy::disallowed_methods)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FoundationError {
    #[error("設定缺少必填項目：{0}")]
    ConfigMissing(&'static str),
}

/// 金鑰專用型別：不能印（Debug 只顯示遮罩）、不能直接序列化（沒有 Serialize）。
/// （`spec/05` §2：金鑰不會被印出來或寫進紀錄，由型別系統擋。）
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    #[must_use]
    pub fn from_untrusted(raw: String) -> Self {
        Secret(raw)
    }

    /// 只有明確要求使用金鑰內容的程式呼叫得到字串。
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(遮罩)")
    }
}

/// 時間（epoch 秒）。假時鐘的注入點之後在 protocol 定介面。
#[must_use]
pub fn now_epoch_seconds() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// foundation 唯一的檔案豁免：讀 `--bootstrap` 指定的啟動檔（`spec/07` §3）。
///
/// # Errors
///
/// 啟動檔讀不到時回傳 `io::Error`。
pub fn read_bootstrap_file(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn secret_is_redacted_in_debug() {
        let s = Secret::from_untrusted("top-secret".to_owned());
        assert_eq!(format!("{s:?}"), "Secret(遮罩)");
        assert_eq!(s.expose(), "top-secret");
    }
}
