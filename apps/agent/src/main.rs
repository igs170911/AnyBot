use std::io::Write;

use clap::Command;

fn main() -> Result<(), std::io::Error> {
    Command::new("anybot-agent")
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 使用者電腦上的背景程式（骨架）")
        .get_matches();
    writeln!(
        std::io::stdout(),
        "anybot-agent {} (M1 骨架，沒有功能)",
        env!("CARGO_PKG_VERSION")
    )
}
