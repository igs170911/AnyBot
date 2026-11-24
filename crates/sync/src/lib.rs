//! sync · 同步：送出匣子、拿第 N 號之後的、SSE 客戶端。
//!
//! 照 `spec/07` §2：發 HTTP 只准發生在這裡。M1 骨架：只定套件邊界，沒有功能邏輯。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
// 打開 clippy.toml 的「發 HTTP 請求」禁用：sync 是准許的套件（spec/07 §3）。
#![allow(clippy::disallowed_methods)]

pub use anybot_foundation::Secret;
use anybot_foundation::now_epoch_seconds;
use anybot_protocol::EventEnvelope;
use anybot_store_client as store_client;

/// 同步客戶端的 HTTP 型別。
pub type SyncHttpClient = reqwest::Client;

#[must_use]
pub fn default_client() -> SyncHttpClient {
    reqwest::Client::new()
}

/// 匣子（打包的同步紀錄）占位型別；合併規則在 domain（純邏輯）。
#[derive(Debug, Clone)]
pub struct BoxedRecords {
    pub pool_ref: Option<store_client::Pool>,
    pub events: Vec<EventEnvelope>,
    pub packed_at_epoch_seconds: i64,
    /// 送匣子到伺服器用的金鑰（Secret：不能印）。
    pub server_token: Secret,
}

impl BoxedRecords {
    #[must_use]
    pub fn pack(events: Vec<EventEnvelope>, server_token: Secret) -> Self {
        BoxedRecords {
            pool_ref: None,
            events,
            packed_at_epoch_seconds: now_epoch_seconds(),
            server_token,
        }
    }
}
