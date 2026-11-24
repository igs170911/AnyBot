//! apps/agent · `AnyBot` 使用者電腦上的背景程式（骨架）
//! 照 `spec/07` §2：main 保持極簡。M1 骨架只印版本、支援 --help，沒有功能。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_sync::BoxedRecords;
use clap::Command;

fn main() {
    let name = env!("CARGO_PKG_NAME");
    let bin = "anybot-agent";
    let matches = Command::new(bin)
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 使用者電腦上的背景程式（骨架）")
        .get_matches();
    // M1 骨架：用到 sync 證明依賴方向接好（背景程式走同步这条路）。
    let _ = (
        name,
        matches,
        BoxedRecords {
            pool_ref: None,
            events: Vec::new(),
            packed_at_epoch_seconds: 0,
            server_token: anybot_sync::Secret::from_untrusted("M1 骨架".to_owned()),
        },
    );
    println!("{bin} {} (M1 骨架，沒有功能)", env!("CARGO_PKG_VERSION"));
}
