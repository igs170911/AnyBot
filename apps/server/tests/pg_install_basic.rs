use tokio::net::TcpListener;

use sqlx as _;

use anybot_app_server as _;
use anybot_app_server::parse_launch;
use anybot_app_test as _;
use anybot_app_test::{ServerDb, ServerProgram, db_start, english};
use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;
use test_harness as _;
use tracing as _;

use anybot_messages::{Catalog, Lang, MessageCode, Params};

use test_harness::{Outcome, Running, pg_stop, run_program, test_dir};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

fn startup_params(version: &str, port: u16) -> Params {
    Params::new()
        .with_text("version", version)
        .with_text("addresses", &format!("127.0.0.1:{port}, [::1]:{port}"))
        .with_text("db_host", "127.0.0.1")
        .with_text("db_name", "anybot_test")
}

fn full_text(catalog: &mut Catalog, code: MessageCode) -> Result<String, String> {
    let mut params = Params::new();
    let names = catalog
        .param_names(Lang::En, &code)
        .map_err(|e| e.to_string())?;
    for name in names {
        if name == "port" {
            params = params.with_int(&name, 31_313);
        } else {
            params = params.with_text(&name, "sample");
        }
    }
    english(catalog, code, &params)
}

fn startup_line_of(db: &ServerDb) -> Result<String, String> {
    english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &startup_params(env!("CARGO_PKG_VERSION"), db.listen_port),
    )
}

fn start_running(db: &ServerDb) -> Result<Running, String> {
    server_bin().start_running(db, &startup_line_of(db)?)
}

#[tokio::test]
async fn t_install_n22_1() -> Result<(), String> {
    let db = db_start("install_n22_1").await?;
    let mut running = start_running(&db)?;
    let result = running.kill();
    let _checked = check_no_password(&result);
    let stopped = pg_stop(&db.pg).await;
    match result {
        Ok(_outcome) => stopped,
        Err(msg) => Err(msg),
    }
}

fn check_no_password(result: &Result<Outcome, String>) -> Result<(), String> {
    if let Ok(outcome) = result
        && outcome.stdout.contains("pgtest")
    {
        return Err("password leaked into stdout".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn t_install_n22_2() -> Result<(), String> {
    let dir = test_dir("install_n22_2")?;
    let outcome = run_program(&server_bin().program(vec!["--help".to_owned()]), &dir)?;
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
                test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Err(format!("failed to hold {addr}: timed out"))
}

async fn hold_port(port: u16) -> Result<(TcpListener, TcpListener), String> {
    let v4 = bind_stack(&format!("127.0.0.1:{port}")).await?;
    let v6 = bind_stack(&format!("[::1]:{port}")).await?;
    Ok((v4, v6))
}

#[tokio::test]
async fn t_install_n22_3() -> Result<(), String> {
    let db = db_start("install_n22_3").await?;
    let held = hold_port(db.listen_port).await?;
    let outcome = run_program(&server_bin().program(db.args()), &db.dir)?;
    let result = check_port_failure(&outcome, db.listen_port);
    drop(held);
    let stopped = pg_stop(&db.pg).await;
    match result {
        Ok(()) => stopped,
        Err(msg) => Err(msg),
    }
}

fn check_port_failure(outcome: &Outcome, port: u16) -> Result<(), String> {
    if outcome.code != 1 {
        return Err(format!("port-conflict exit {}", outcome.code));
    }
    if !outcome.stdout.is_empty() {
        return Err(format!("port-conflict server printed stdout {outcome:?}"));
    }
    let want = format!("Cannot use port {port}:");
    if !outcome.stderr.contains(&want) {
        return Err(format!("{outcome:?} misses {want}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_install_n22_4() -> Result<(), String> {
    let dir = test_dir("install_n22_4")?;
    let no_file = run_program(&server_bin().program(vec![]), &dir)?;
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
    let argv = vec![
        "--bootstrap".to_owned(),
        path.display().to_string(),
        "--port".to_owned(),
        "1".to_owned(),
    ];
    let unknown = run_program(&server_bin().program(argv.clone()), &dir)?;
    if unknown.code != 2 {
        return Err(format!("unknown-flag exit {}", unknown.code));
    }
    let expected = arguments_invalid_text(&argv)?;
    if !unknown.stderr.contains(&expected) {
        return Err(format!("{unknown:?} misses {expected}"));
    }
    Ok(())
}

fn arguments_invalid_text(argv: &[String]) -> Result<String, String> {
    let failure = match parse_launch(argv) {
        Err(failure) => failure,
        Ok(parsed) => return Err(format!("fixture argv parsed without failure: {parsed:?}")),
    };
    if failure.code != MessageCode::ServerArgumentsInvalid {
        return Err(format!("fixture code mismatch: {:?}", failure.code));
    }
    english(&mut Catalog::new(), failure.code, &failure.params)
}
