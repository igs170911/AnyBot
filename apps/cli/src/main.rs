use std::io::Write;

use clap::Command;

fn main() -> Result<(), std::io::Error> {
    Command::new("anybot")
        .version(env!("CARGO_PKG_VERSION"))
        .about("AnyBot 命令列：安裝、遷移、診斷（骨架）")
        .get_matches();
    writeln!(
        std::io::stdout(),
        "anybot {} (M1 骨架，沒有功能)",
        env!("CARGO_PKG_VERSION")
    )
}
