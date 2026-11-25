use axum as _;
use std::io::Write;
use std::process::ExitCode;
#[cfg(test)]
use test_harness as _;

use anybot_server::serve_all::serve_all;
use anybot_server::startup::DEFAULT_PORT;
use anybot_server::{AppState, build_router};

use anybot_app_server::{PlanFailure, bind_plan, parse_launch, startup_plan};

use anybot_foundation as _;
use clap as _;

fn print_line(line: &str) {
    let mut stdout = std::io::stdout();
    let _result = writeln!(stdout, "{line}");
}

fn print_problem(message: &str) {
    let mut stderr = std::io::stderr();
    let _result = writeln!(stderr, "{message}");
}

fn report_failure(failure: &PlanFailure) -> ExitCode {
    if failure.code == ExitCode::SUCCESS {
        print_line(&failure.message);
        return failure.code;
    }
    print_problem(&failure.message);
    failure.code
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_launch(&raw) {
        Ok(args) => args,
        Err(failure) => return report_failure(&failure),
    };
    let version = env!("CARGO_PKG_VERSION");
    if let Some(command) = &args.command {
        print_problem(format!("這個子指令還沒有實作：{command:?}").as_str());
        return ExitCode::from(2);
    }
    let startup = match startup_plan(&args, version) {
        Ok(startup) => startup,
        Err(failure) => return report_failure(&failure),
    };
    print_line(&startup.startup_line);
    let listeners = match bind_plan(DEFAULT_PORT).await {
        Ok(listeners) => listeners,
        Err(failure) => return report_failure(&failure),
    };
    let app = build_router(AppState::new(version));
    let reason = match serve_all(listeners, app).await {
        Ok(reason) | Err(reason) => reason,
    };
    print_problem(&reason);
    ExitCode::from(1)
}
