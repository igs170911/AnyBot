//! apps/cli · `AnyBot` 命令列：安裝、遷移、診斷（骨架）
//! 照 `spec/07` §2：main 保持極簡。M1 骨架只印版本、支援 --help，沒有功能。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use anybot_foundation::now_epoch_seconds;
use clap::Command;

fn main() {
    let name = env!("CARGO_PKG_NAME");
    let bin = "anybot";
    let matches = Command::new(bin)
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 命令列：安裝、遷移、診斷（骨架）")
        .get_matches();
    // M1 骨架：用到 foundation 證明依賴方向接好。
    let _ = (name, matches, now_epoch_seconds());
    println!("{bin} {} (M1 骨架，沒有功能)", env!("CARGO_PKG_VERSION"));
}
