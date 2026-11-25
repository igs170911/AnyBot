use std::io::Write;
use std::process::ExitCode;

use clap::Command;

use anybot_messages as _;
use anybot_messages::{Catalog, MessageCode, ParamValue, Params};

fn main() -> ExitCode {
    let mut catalog = Catalog::new();
    Command::new("anybot-runner-service")
        .version(env!("CARGO_PKG_VERSION"))
        .about("runner service on the server (skeleton)")
        .get_matches();
    let params = Params::single("version", ParamValue::text(env!("CARGO_PKG_VERSION")));
    match catalog.format_default(&MessageCode::RunnerServiceSkeletonBanner, &params) {
        Ok(text) => match writeln!(std::io::stdout(), "{text}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Err(_) => ExitCode::FAILURE,
    }
}
