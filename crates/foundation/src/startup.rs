use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::error::FoundationError;

pub const APP_IDENTIFIER: &str = "app.anybot.desktop";

#[derive(Debug, Parser)]
#[command(name = "anybot-client", version)]
pub struct ClientLaunchArgs {
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ServerCommand {
    ResetPassword { account: String },
    Logs,
    UpdateBotComputers,
    MigrateOld,
}

#[derive(Debug, Parser)]
#[command(name = "anybot-server", version)]
pub struct ServerLaunchArgs {
    #[arg(long)]
    pub bootstrap: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<ServerCommand>,
}

#[derive(Debug)]
pub struct ClientLaunch {
    pub data_dir: PathBuf,
}

impl ClientLaunch {
    pub fn from_args(args: &ClientLaunchArgs) -> Result<Self, FoundationError> {
        match &args.data_dir {
            Some(dir) => Ok(ClientLaunch {
                data_dir: dir.clone(),
            }),
            None => default_data_dir()
                .map(|data_dir| ClientLaunch { data_dir })
                .ok_or(FoundationError::DataDirMissing(
                    "找不到本機資料夾的慣例位置：請用 --data-dir 指定".to_owned(),
                )),
        }
    }
}

#[must_use]
pub fn default_data_dir() -> Option<PathBuf> {
    let home = std::env::home_dir()?;
    let rel = match std::env::consts::OS {
        "macos" => "Library/Application Support",
        "linux" => ".local/share",
        "windows" => "AppData/Roaming",
        _ => return None,
    };
    let mut dir = home;
    for part in rel.split('/') {
        dir.push(part);
    }
    dir.push(APP_IDENTIFIER);
    Some(dir)
}
