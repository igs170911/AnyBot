#[cfg(test)]
use anybot_app_test as _;
use axum as _;
use serde_json as _;
use sqlx as _;
use std::process::ExitCode;
#[cfg(test)]
use test_harness as _;
#[cfg(test)]
use tracing as _;

use anybot_app_server::{Ctx, Parsed, emit, parse_launch, server_run, subcommand_plan};
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use clap as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut ctx = Ctx::new();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_launch(&raw) {
        Ok(Parsed::Args(args)) => args,
        Ok(Parsed::Printed(text)) => {
            let _result = emit(&text, &mut ctx.stdout);
            return ExitCode::SUCCESS;
        }
        Err(failure) => return ctx.report(&failure),
    };
    if let Some(failure) = subcommand_plan(&args) {
        return ctx.report(&failure);
    }
    server_run::serve_main(&args, env!("CARGO_PKG_VERSION"), &mut ctx).await
}
