use std::io::Write;
use std::process::ExitCode;

use clap::Command;

use anybot_foundation::version_string;
use anybot_messages as _;
use anybot_messages::{Catalog, MessageCode, ParamValue, Params};

fn main() -> ExitCode {
    let mut catalog = Catalog::new();
    let version = version_string();
    if let Some(code) = print_requested_version(&version) {
        return code;
    }
    let matches = Command::new("anybot-runner-service")
        .disable_version_flag(true)
        .about("runner service on the server (skeleton)")
        .get_matches();
    let _held = matches;
    let params = Params::single("version", ParamValue::text(&version));
    match catalog.format_default(&MessageCode::RunnerServiceSkeletonBanner, &params) {
        Ok(text) => match writeln!(std::io::stdout(), "{text}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Err(_) => ExitCode::FAILURE,
    }
}

fn print_requested_version(version: &str) -> Option<ExitCode> {
    let requested = std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V");
    if !requested {
        return None;
    }
    match writeln!(std::io::stdout(), "{version}") {
        Ok(()) => Some(ExitCode::SUCCESS),
        Err(_) => Some(ExitCode::FAILURE),
    }
}
