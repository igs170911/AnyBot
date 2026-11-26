#[cfg(test)]
use anybot_domain as _;
#[cfg(test)]
use anybot_server_test as _;

#[cfg(test)]
use anybot_app_test as _;
use std::io::Write;

use axum as _;
use serde_json as _;
use sqlx as _;
use std::process::ExitCode;
#[cfg(test)]
use test_harness as _;
#[cfg(test)]
use tracing as _;

use anybot_app_server::{Ctx, Parsed, emit, parse_launch, server_run, subcommand_plan};
use anybot_foundation::{self as _, ServerCommand, log_version, version_string};
use anybot_messages as _;
use anybot_server as _;
use anybot_store_server as _;
use clap as _;
use rpassword as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut ctx = Ctx::new();
    let version = version_string();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|arg| arg == "--version" || arg == "-V") {
        return print_version(&version);
    }
    let args = match parse_launch(&raw) {
        Ok(Parsed::Args(args)) => args,
        Ok(Parsed::Printed(text)) => {
            let _result = emit(&text, &mut ctx.stdout);
            return ExitCode::SUCCESS;
        }
        Err(failure) => return ctx.report(&failure),
    };
    if let Some(ServerCommand::ResetPassword { account }) = &args.command {
        return anybot_app_server::cli_reset::reset_main(
            &args,
            log_version(&version),
            account,
            &mut ctx,
        )
        .await;
    }
    if let Some(failure) = subcommand_plan(&args) {
        return ctx.report(&failure);
    }
    server_run::serve_main(&args, log_version(&version), &mut ctx).await
}

fn print_version(version: &str) -> ExitCode {
    match writeln!(std::io::stdout(), "{version}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
