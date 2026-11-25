use std::io::Write;

use axum as _;
use serde_json as _;
use std::process::ExitCode;
#[cfg(test)]
use test_harness as _;

use anybot_app_server::{
    Parsed, bind_plan, emit, parse_launch, report_failure, serve_exit, startup_plan, startup_text,
    subcommand_plan,
};
use anybot_foundation as _;
use anybot_messages::Catalog;
use anybot_server::startup::DEFAULT_PORT;
use anybot_server::{AppState, build_router};
use clap as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut catalog = Catalog::new();
    let version = anybot_foundation::version_string();
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.iter().any(|arg| arg == "--version" || arg == "-V") {
        return print_version(&version);
    }
    let args = match parse_launch(&raw) {
        Ok(Parsed::Args(args)) => args,
        Ok(Parsed::Printed(text)) => {
            let _result = emit(&text, &mut std::io::stdout());
            return ExitCode::SUCCESS;
        }
        Err(failure) => {
            return report_failure(&mut catalog, &failure, &mut std::io::stderr());
        }
    };
    if let Some(failure) = subcommand_plan(&args) {
        return report_failure(&mut catalog, &failure, &mut std::io::stderr());
    }
    let startup = match startup_plan(&args, anybot_foundation::log_version(&version)) {
        Ok(Ok(startup)) => startup,
        Ok(Err(failure)) => return report_failure(&mut catalog, &failure, &mut std::io::stderr()),
        Err(err) => return bug_exit(&mut catalog, &err),
    };
    let listeners = match bind_plan(DEFAULT_PORT).await {
        Ok(listeners) => listeners,
        Err(failure) => return report_failure(&mut catalog, &failure, &mut std::io::stderr()),
    };
    if startup_text(&mut catalog, &startup).is_err() {
        return ExitCode::from(3);
    }
    let app = build_router(AppState::new(&version));
    serve_exit(&mut catalog, listeners, app).await
}

fn print_version(version: &str) -> ExitCode {
    match writeln!(std::io::stdout(), "{version}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn bug_exit(_catalog: &mut Catalog, err: &anybot_messages::MessagesError) -> ExitCode {
    let _result = emit(&err.to_string(), &mut std::io::stderr());
    ExitCode::from(3)
}
