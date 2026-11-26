use std::process::ExitCode;

use tokio::net::TcpListener;

use anybot_app_server::{Parsed, bind_plan, help_text, parse_launch, startup_line, startup_plan};

use anybot_foundation as _;
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;

use test_harness::{Program, run_program, test_dir, write_file_with_mode};

const GOOD_BOOTSTRAP: &str =
    "postgresql://anybot:fixture-not-a-secret@localhost:5432/anybot_test\n";

fn server_program(args: Vec<String>) -> Program {
    Program::new(env!("CARGO_BIN_EXE_anybot-server"), args)
}

fn bootstrap_args(path: &std::path::Path) -> Vec<String> {
    vec!["--bootstrap".to_owned(), path.display().to_string()]
}

fn scratch_bootstrap(test: &str, mode: u32) -> Result<std::path::PathBuf, String> {
    let dir = test_dir(test)?;
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, GOOD_BOOTSTRAP, mode)?;
    Ok(path)
}

fn english(catalog: &mut Catalog, code: MessageCode, params: &Params) -> Result<String, String> {
    catalog
        .format(Lang::En, &code, params)
        .map_err(|e| e.to_string())
}

fn plan_text(
    result: Result<
        Result<anybot_app_server::AppServerStartup, anybot_app_server::PlanFailure>,
        anybot_messages::MessagesError,
    >,
) -> Result<Result<anybot_app_server::AppServerStartup, anybot_app_server::PlanFailure>, String> {
    result.map_err(|e| e.to_string())
}

fn args_from(argv: &[String]) -> Result<anybot_foundation::ServerLaunchArgs, String> {
    match parse_launch(argv) {
        Ok(Parsed::Args(launch)) => Ok(launch),
        Ok(Parsed::Printed(text)) => Err(format!("expected args, got printed text {text}")),
        Err(failure) => Err(format!("parse failed with exit {:?}", failure.exit)),
    }
}

#[test]
fn startup_line_reports_both_addresses_without_password() -> Result<(), String> {
    let path = scratch_bootstrap("unit_startup_line", 0o600)?;
    let args = args_from(&bootstrap_args(&path))?;
    let startup = match plan_text(startup_plan(&args, "9.9.9"))? {
        Ok(startup) => startup,
        Err(failure) => return Err(format!("startup plan rejected: {:?}", failure.code)),
    };
    let line = startup_line(&startup).map_err(|e| e.to_string())?;
    if line.code != MessageCode::ServerStartupListening {
        return Err(format!("unexpected code {:?}", line.code));
    }
    let text = english(&mut Catalog::new(), line.code, &line.params)?;
    for want in [
        "v9.9.9",
        "127.0.0.1:8787",
        "[::1]:8787",
        "localhost/anybot_test",
    ] {
        if !text.contains(want) {
            return Err(format!("missing {want} in {text}"));
        }
    }
    if text.contains("fixture-not-a-secret") {
        return Err("password leaked into the startup line".to_owned());
    }
    let json = serde_json::to_string(&startup.startup.params).map_err(|e| e.to_string())?;
    if json.contains("fixture-not-a-secret") {
        return Err("password leaked into the params json".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn bind_plan_says_port_unusable_when_port_is_taken() -> Result<(), String> {
    for _attempt in 0..40 {
        match TcpListener::bind("127.0.0.1:8787").await {
            Ok(held) => {
                let result = bind_plan(8787).await;
                let outcome = check_bind_failure(&held, result);
                drop(held);
                return outcome;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
            }
            Err(err) => return Err(format!("bind failed: {err}")),
        }
    }
    Err("failed to hold port 8787 in time".to_owned())
}

fn check_bind_failure(
    _held: &TcpListener,
    result: Result<Vec<TcpListener>, anybot_app_server::PlanFailure>,
) -> Result<(), String> {
    let failure = match result {
        Ok(_listeners) => return Err("bind plan accepted a taken port".to_owned()),
        Err(failure) => failure,
    };
    if failure.exit != ExitCode::from(1) {
        return Err(format!("unexpected exit {:?}", failure.exit));
    }
    if failure.code != MessageCode::ServerPortUnusable {
        return Err(format!("unexpected code {:?}", failure.code));
    }
    let want_port = anybot_messages::ParamValue::int(8787);
    if failure.params.get("port") != Some(&want_port) {
        return Err(format!("unexpected params {:?}", failure.params));
    }
    let text = english(&mut Catalog::new(), failure.code, &failure.params)?;
    if !text.starts_with("Cannot use port 8787:") {
        return Err(format!("unexpected text {text}"));
    }
    Ok(())
}

#[test]
fn startup_plan_says_bootstrap_flag_when_file_is_missing() -> Result<(), String> {
    let result = plan_text(startup_plan(&args_from(&[])?, "9.9.9"))?;
    let failure = match result {
        Ok(_startup) => return Err("startup plan accepted a missing bootstrap".to_owned()),
        Err(failure) => failure,
    };
    if failure.exit != ExitCode::from(2) {
        return Err(format!("unexpected exit {:?}", failure.exit));
    }
    if failure.code != MessageCode::ServerBootstrapMissing {
        return Err(format!("unexpected code {:?}", failure.code));
    }
    let text = english(&mut Catalog::new(), failure.code, &failure.params)?;
    if !text.contains("--bootstrap") {
        return Err(format!("missing --bootstrap in {text}"));
    }
    Ok(())
}

#[test]
fn parse_launch_rejects_unknown_flag_with_exit_code_two() -> Result<(), String> {
    let result = parse_launch(&[
        "--bootstrap".to_owned(),
        "x.conf".to_owned(),
        "--port".to_owned(),
        "1".to_owned(),
    ]);
    let failure = match result {
        Ok(parsed) => return Err(format!("unexpected parse result {parsed:?}")),
        Err(failure) => failure,
    };
    if failure.exit != ExitCode::from(2) {
        return Err(format!("unexpected exit {:?}", failure.exit));
    }
    if failure.code != MessageCode::ServerArgumentsInvalid {
        return Err(format!("unexpected code {:?}", failure.code));
    }
    if failure.params.get("reason").is_none() {
        return Err(format!("missing reason param {:?}", failure.params));
    }
    let text = english(&mut Catalog::new(), failure.code, &failure.params)?;
    if !text.starts_with("Command line arguments are invalid:") {
        return Err(format!("unexpected text {text}"));
    }
    Ok(())
}

#[test]
fn help_text_lists_all_subcommands_and_flags() -> Result<(), String> {
    let help = help_text();
    for want in ["reset-password", "--bootstrap", "--version"] {
        if !help.contains(want) {
            return Err(format!("missing {want} in help"));
        }
    }
    Ok(())
}

#[test]
fn startup_plan_rejects_world_readable_bootstrap() -> Result<(), String> {
    let path = scratch_bootstrap("unit_permissions", 0o644)?;
    let result = plan_text(startup_plan(&args_from(&bootstrap_args(&path))?, "9.9.9"))?;
    let failure = match result {
        Ok(_startup) => return Err("startup plan accepted a world-readable bootstrap".to_owned()),
        Err(failure) => failure,
    };
    if failure.exit == ExitCode::SUCCESS {
        return Err("success exit for a rejected file".to_owned());
    }
    if failure.code != MessageCode::ServerBootstrapPermissionsTooOpen {
        return Err(format!("unexpected code {:?}", failure.code));
    }
    let want_mode = anybot_messages::ParamValue::text("644");
    if failure.params.get("mode") != Some(&want_mode) {
        return Err(format!("unexpected params {:?}", failure.params));
    }
    let text = english(&mut Catalog::new(), failure.code, &failure.params)?;
    if !text.contains("644") {
        return Err(format!("missing 644 in {text}"));
    }
    Ok(())
}

#[test]
fn parse_launch_prints_help_and_version_text_with_success() -> Result<(), String> {
    let help = match parse_launch(&["--help".to_owned()]) {
        Ok(Parsed::Printed(text)) => text,
        other => return Err(format!("expected printed help, got {other:?}")),
    };
    if !help.contains("reset-password") {
        return Err(format!("missing reset-password in {help}"));
    }
    let version = match parse_launch(&["--version".to_owned()]) {
        Ok(Parsed::Printed(text)) => text,
        other => return Err(format!("expected printed version, got {other:?}")),
    };
    if !version.contains("anybot-server") && !version.starts_with('v') {
        return Err(format!("missing version text in {version}"));
    }
    Ok(())
}

#[test]
fn help_and_version_print_to_stdout_with_zero_exit() -> Result<(), String> {
    let dir = test_dir("unit_help_stdout")?;
    let help = run_program(&server_program(vec!["--help".to_owned()]), &dir)?;
    if help.code != 0 {
        return Err(format!("help exit {}", help.code));
    }
    if help.stderr.contains("Usage:") {
        return Err("usage text landed on stderr".to_owned());
    }
    let version = run_program(&server_program(vec!["--version".to_owned()]), &dir)?;
    if version.code != 0 {
        return Err(format!("version exit {}", version.code));
    }
    if !version.stdout.starts_with('v') || !version.stdout.contains(" · ") {
        return Err(format!("unexpected version output {version:?}"));
    }
    Ok(())
}
