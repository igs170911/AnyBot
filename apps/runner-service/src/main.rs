use std::io::Write;

use clap::Command;

fn main() -> Result<(), std::io::Error> {
    Command::new("anybot-runner-service")
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 伺服器上的執行服務（骨架）")
        .get_matches();
    writeln!(
        std::io::stdout(),
        "anybot-runner-service {} (M1 骨架，沒有功能)",
        env!("CARGO_PKG_VERSION")
    )
}
