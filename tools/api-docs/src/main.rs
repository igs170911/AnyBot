use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use api_docs::{asyncapi, openapi};

use anybot_protocol as _;
use anybot_server as _;
use noyalib as _;
use schemars as _;
use serde_json as _;
use utoipa as _;

#[derive(Parser)]
#[command(
    name = "api-docs",
    version,
    about = "產生 api/ 底下的文件（spec/02 §8）"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Openapi { path: String },
    Asyncapi { path: String },
}

fn write_file(path: &str, content: &str) -> Result<(), std::io::Error> {
    let parent = std::path::Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = parent {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let generated = match args.command {
        Command::Openapi { .. } => openapi::document(),
        Command::Asyncapi { .. } => asyncapi::document(),
    };
    let content = match generated {
        Ok(content) => content,
        Err(err) => return report(&format!("產生失敗：{err}")),
    };
    let path = match args.command {
        Command::Openapi { path } | Command::Asyncapi { path } => path,
    };
    match write_file(&path, &content) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report(&format!("寫檔失敗：{err}")),
    }
}

fn report(message: &str) -> ExitCode {
    let mut stderr = std::io::stderr();
    let _result = writeln!(stderr, "{message}");
    ExitCode::from(1)
}
