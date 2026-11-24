//! apps/runner-service · `AnyBot` 伺服器上的執行服務（骨架）
//! 照 `spec/07` §2：main 保持極簡。M1 骨架只印版本、支援 --help，沒有功能。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_protocol::EventEnvelope;
use clap::Command;

fn main() {
    let name = env!("CARGO_PKG_NAME");
    let bin = "anybot-runner-service";
    let matches = Command::new(bin)
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 伺服器上的執行服務（骨架）")
        .get_matches();
    // M1 骨架：用到 protocol 證明依賴方向接好（執行服務也是 API 的客戶端）。
    let _ = (name, matches, EventEnvelope::now(0, "skeleton"));
    println!("{bin} {} (M1 骨架，沒有功能)", env!("CARGO_PKG_VERSION"));
}
