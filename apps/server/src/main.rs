use std::io::Write;

use clap::Command;

fn main() -> Result<(), std::io::Error> {
    Command::new("anybot-server")
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 伺服器 API（骨架）")
        .get_matches();
    writeln!(
        std::io::stdout(),
        "anybot-server {} (M1 骨架，沒有功能)",
        env!("CARGO_PKG_VERSION")
    )
}
