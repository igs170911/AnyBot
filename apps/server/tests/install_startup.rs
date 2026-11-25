use tokio::net::TcpListener;

use anybot_app_server as _;
use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;

use anybot_messages::{Catalog, Lang, MessageCode, Params};

use test_harness::{Program, Running, http_get, run_program, test_dir, write_file_with_mode};

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

fn parent_dir(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    match path.parent() {
        Some(dir) => Ok(dir.to_path_buf()),
        None => Err("bootstrap file must sit in a directory".to_owned()),
    }
}

fn english(catalog: &mut Catalog, code: MessageCode, params: &Params) -> Result<String, String> {
    catalog
        .format(Lang::En, &code, params)
        .map_err(|e| e.to_string())
}

fn full_text(catalog: &mut Catalog, code: MessageCode) -> Result<String, String> {
    let mut params = Params::new();
    let names = catalog
        .param_names(Lang::En, &code)
        .map_err(|e| e.to_string())?;
    for name in names {
        if name == "port" {
            params = params.with_int(&name, 8787);
        } else {
            params = params.with_text(&name, "sample");
        }
    }
    english(catalog, code, &params)
}

fn startup_params(version: &str) -> Params {
    Params::new()
        .with_text("version", version)
        .with_int("port", 8787)
        .with_text("db_host", "localhost")
        .with_text("db_name", "anybot_test")
}

#[tokio::test]
async fn t_install_n22_1() -> Result<(), String> {
    let path = scratch_bootstrap("install_n22_1", 0o600)?;
    let dir = parent_dir(&path)?;
    let mut running = Running::start(&server_program(bootstrap_args(&path)), &dir)?;
    let first_line = running.next_stdout_line()?;
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &startup_params(env!("CARGO_PKG_VERSION")),
    )?;
    if first_line.trim() != expected {
        return Err(format!("startup line {first_line} differs from {expected}"));
    }
    if first_line.contains("fixture-not-a-secret") {
        return Err("password leaked into the startup line".to_owned());
    }
    let _result = running.kill()?;
    Ok(())
}

#[tokio::test]
async fn t_install_n22_2() -> Result<(), String> {
    let dir = test_dir("install_n22_2")?;
    let outcome = run_program(&server_program(vec!["--help".to_owned()]), &dir)?;
    if outcome.code != 0 {
        return Err(format!("help exit {}", outcome.code));
    }
    for want in [
        "reset-password",
        "logs",
        "update-bot-computers",
        "migrate-old",
        "--bootstrap",
        "--version",
    ] {
        if !outcome.stdout.contains(want) {
            return Err(format!("missing {want} in help output"));
        }
    }
    Ok(())
}

async fn bind_stack(addr: &str) -> Result<TcpListener, String> {
    for _attempt in 0..40 {
        match TcpListener::bind(addr).await {
            Ok(listener) => return Ok(listener),
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Err(format!("failed to hold {addr}: timed out"))
}

async fn hold_port_8787() -> Result<(TcpListener, TcpListener), String> {
    let v4 = bind_stack("127.0.0.1:8787").await?;
    let v6 = bind_stack("[::1]:8787").await?;
    Ok((v4, v6))
}

#[tokio::test]
async fn t_install_n22_3() -> Result<(), String> {
    let held = hold_port_8787().await?;
    let dir = test_dir("install_n22_3")?;
    let path = scratch_bootstrap("install_n22_3", 0o600)?;
    let outcome = run_program(&server_program(bootstrap_args(&path)), &dir)?;
    let result = check_port_failure(&held, &outcome);
    drop(held);
    result
}

fn check_port_failure(
    _held: &(TcpListener, TcpListener),
    outcome: &test_harness::Outcome,
) -> Result<(), String> {
    if outcome.code != 1 {
        return Err(format!("port-conflict exit {}", outcome.code));
    }
    let expected = full_text(&mut Catalog::new(), MessageCode::ServerPortUnusable)?;
    let Some(prefix) = expected.split(':').next() else {
        return Err("port message has no first segment".to_owned());
    };
    if !outcome.stderr.contains(prefix) {
        return Err(format!("{outcome:?} misses {prefix}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_install_n22_4() -> Result<(), String> {
    let dir = test_dir("install_n22_4")?;
    let no_file = run_program(&server_program(vec![]), &dir)?;
    if no_file.code != 2 {
        return Err(format!("missing-bootstrap exit {}", no_file.code));
    }
    let expected = full_text(&mut Catalog::new(), MessageCode::ServerBootstrapMissing)?;
    if !no_file.stderr.contains(&expected) {
        return Err(format!("{no_file:?} misses {expected}"));
    }
    if !no_file.stderr.contains("--bootstrap") {
        return Err("missing --bootstrap hint".to_owned());
    }
    let path = dir.join("x.conf");
    let unknown = run_program(
        &server_program(vec![
            "--bootstrap".to_owned(),
            path.display().to_string(),
            "--port".to_owned(),
            "1".to_owned(),
        ]),
        &dir,
    )?;
    if unknown.code != 2 {
        return Err(format!("unknown-flag exit {}", unknown.code));
    }
    Ok(())
}

#[tokio::test]
async fn t_server_n14_2() -> Result<(), String> {
    let path = scratch_bootstrap("server_n14_2", 0o644)?;
    let dir = parent_dir(&path)?;
    let outcome = run_program(&server_program(bootstrap_args(&path)), &dir)?;
    if outcome.code == 0 {
        return Err("world-readable bootstrap was accepted".to_owned());
    }
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerBootstrapPermissionsTooOpen,
        &Params::new()
            .with_text("mode", "644")
            .with_text("path", &path.display().to_string()),
    )?;
    if !outcome.stderr.contains(&expected) {
        return Err(format!("{outcome:?} misses {expected}"));
    }
    Ok(())
}

async fn start_healthy_server(
    path: &std::path::Path,
    dir: &std::path::Path,
) -> Result<Running, String> {
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &startup_params(env!("CARGO_PKG_VERSION")),
    )?;
    let mut running = Running::start(&server_program(bootstrap_args(path)), dir)?;
    let line = running.next_stdout_line()?;
    if line.trim() != expected {
        return Err(format!("startup line {line} differs from {expected}"));
    }
    for _attempt in 0..40 {
        let (code, _body) = http_get(8787, "/v1/health").await?;
        if code == 200 {
            return Ok(running);
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err("health endpoint never answered 200".to_owned())
}

#[tokio::test]
async fn health_endpoint_answers_on_the_started_server() -> Result<(), String> {
    let path = scratch_bootstrap("unit_health", 0o600)?;
    let dir = parent_dir(&path)?;
    let mut running = start_healthy_server(&path, &dir).await?;
    let (code, _body) = http_get(8787, "/v1/health").await?;
    if code != 200 {
        return Err(format!("health status {code}"));
    }
    let _result = running.kill()?;
    Ok(())
}

#[tokio::test]
async fn second_server_exits_with_one_while_first_keeps_serving_health() -> Result<(), String> {
    let path = scratch_bootstrap("two_servers_second_fails", 0o600)?;
    let dir = parent_dir(&path)?;
    let mut first = start_healthy_server(&path, &dir).await?;
    let second = run_program(&server_program(bootstrap_args(&path)), &dir)?;
    if second.code != 1 {
        return Err(format!("second server exit {}", second.code));
    }
    if !second.stderr.contains("Cannot use port 8787") {
        return Err(format!("{second:?} misses the port message"));
    }
    let (code, _body) = http_get(8787, "/v1/health").await?;
    if code != 200 {
        return Err(format!("first server health status {code}"));
    }
    let _result = first.kill()?;
    Ok(())
}
