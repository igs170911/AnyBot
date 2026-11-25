use std::process::ExitCode;

use tokio::net::TcpListener;

use anybot_foundation::ServerLaunchArgs;
use anybot_server::startup::{ServerStartup, bind_listeners, prepare};
use clap::{CommandFactory, Parser};

use axum as _;

#[derive(Debug)]
pub struct PlanFailure {
    pub message: String,
    pub code: ExitCode,
}

impl PlanFailure {
    #[must_use]
    pub fn new(message: String, code: u8) -> Self {
        PlanFailure {
            message,
            code: ExitCode::from(code),
        }
    }
}

impl From<PlanFailure> for String {
    fn from(failure: PlanFailure) -> Self {
        failure.message
    }
}

#[must_use]
pub fn help_text() -> String {
    ServerLaunchArgs::command().render_help().to_string()
}

pub fn parse_launch(argv: &[String]) -> Result<ServerLaunchArgs, PlanFailure> {
    let mut argv0 = vec!["anybot-server".to_owned()];
    argv0.extend_from_slice(argv);
    ServerLaunchArgs::try_parse_from(argv0).map_err(|err| PlanFailure {
        message: err.to_string(),
        code: clap_exit_code(&err),
    })
}

fn clap_exit_code(err: &clap::Error) -> ExitCode {
    let kind = err.kind();
    if kind == clap::error::ErrorKind::DisplayHelp || kind == clap::error::ErrorKind::DisplayVersion
    {
        return ExitCode::SUCCESS;
    }
    ExitCode::from(2)
}

pub fn startup_plan(args: &ServerLaunchArgs, version: &str) -> Result<ServerStartup, PlanFailure> {
    prepare(args, version).map_err(|err| PlanFailure::new(format!("{err}"), 2))
}

pub async fn bind_plan(port: u16) -> Result<Vec<TcpListener>, PlanFailure> {
    bind_listeners(port)
        .await
        .map_err(|err| PlanFailure::new(format!("無法使用連接埠 {port}：{err}"), 1))
}
