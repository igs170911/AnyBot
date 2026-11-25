use tokio::net::TcpListener;

use sqlx as _;

use anybot_app_server::parse_launch;
use anybot_foundation as _;
use anybot_server as _;
use axum as _;
use clap as _;
use serde_json as _;

use anybot_messages::{Catalog, Lang, MessageCode, Params};

use test_harness::{
    Outcome, Program, Running, ServerDb, db_start, http_get, pg_stop, run_program, test_dir,
    write_file_with_mode,
};

fn server_program(args: Vec<String>) -> Program {
    Program::new(env!("CARGO_BIN_EXE_anybot-server"), args)
}

fn bootstrap_args(path: &std::path::Path) -> Vec<String> {
    vec!["--bootstrap".to_owned(), path.display().to_string()]
}

fn startup_params(version: &str, port: u16) -> Params {
    Params::new()
        .with_text("version", version)
        .with_text("addresses", &format!("127.0.0.1:{port}, [::1]:{port}"))
        .with_text("db_host", "127.0.0.1")
        .with_text("db_name", "anybot_test")
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
    let expected = startup_line_of(db)?;
    let mut running = Running::start(&server_program(db.args()), &db.dir)?;
    let announced = running.next_stdout_line()?;
    if announced.trim() != expected {
        return Err(format!("startup line {announced} differs from {expected}"));
    }
    Ok(running)
}

async fn wait_health(db: &ServerDb) -> Result<(), String> {
    for _attempt in 0..40 {
        let (code, _body) = http_get(db.listen_port, "/v1/health").await?;
        if code == 200 {
            return Ok(());
        }
        test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
    }
    Err("health endpoint never answered 200".to_owned())
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
    let outcome = run_program(&server_program(db.args()), &db.dir)?;
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
    let argv = vec![
        "--bootstrap".to_owned(),
        path.display().to_string(),
        "--port".to_owned(),
        "1".to_owned(),
    ];
    let unknown = run_program(&server_program(argv.clone()), &dir)?;
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

#[tokio::test]
async fn t_server_n14_2() -> Result<(), String> {
    let dir = test_dir("server_n14_2")?;
    let path = dir.join("boot.conf");
    write_file_with_mode(&path, "postgresql://x:y@127.0.0.1:5432/z\n", 0o644)?;
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

#[tokio::test]
async fn t_server_n14_1_public_url_survives_a_restart() -> Result<(), String> {
    let db = db_start("server_n14_1").await?;
    let first = start_live(&db).await?;
    let _stopped = stop_live(first, db.listen_port).await;
    set_public_url(&db.url, "https://next.example/").await?;
    let second = start_live(&db).await?;
    let body = get_status(&db).await;
    let result = stop_live(second, db.listen_port).await;
    let stopped = pg_stop(&db.pg).await;
    match check_both(body, result, stopped) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn check_both(
    body: Result<String, String>,
    stopped_now: Result<(), String>,
    stopped_db: Result<(), String>,
) -> Result<(), String> {
    match body {
        Ok(text) => match public_url_check(&text) {
            Ok(()) => match stopped_now {
                Ok(()) => stopped_db,
                Err(msg) => Err(msg),
            },
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}

async fn set_public_url(url: &str, value: &str) -> Result<(), String> {
    let pool = anybot_server::open_url(url)
        .await
        .map_err(|e| e.to_string())?;
    anybot_server::write_config_row(&pool, "public_url", value)
        .await
        .map_err(|e| e.to_string())
}

async fn get_status(db: &ServerDb) -> Result<String, String> {
    let (code, body) = http_get(db.listen_port, "/v1/server/status").await?;
    if code != 200 {
        return Err(format!("status code {code}"));
    }
    Ok(body)
}

fn public_url_check(body: &str) -> Result<(), String> {
    if !body.contains("https://next.example/") {
        return Err(format!("status body misses public url: {body}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_server_n14_4_environment_variables_change_nothing() -> Result<(), String> {
    let db = db_start("server_n14_4").await?;
    let mut program = server_program(db.args());
    program = program.with_env("ANYBOT_RELAY_PUBLIC_URL", "https://env.example/");
    program = program.with_env("ANYBOT_RELAY_INVITE", "zzzz-zzzz");
    let mut running = Running::start(&program, &db.dir)?;
    let announced = running.next_stdout_line()?;
    let expected = startup_line_of(&db)?;
    let body = get_status(&db).await;
    let _result = running.kill();
    let stopped = pg_stop(&db.pg).await;
    match env_check(&announced, &expected, body, stopped) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn env_check(
    announced: &str,
    expected: &str,
    body: Result<String, String>,
    stopped: Result<(), String>,
) -> Result<(), String> {
    if announced.trim() != expected {
        return Err(format!("startup line {announced} differs from {expected}"));
    }
    match body {
        Ok(text) => {
            if text.contains("env.example") || text.contains("zzzz") {
                return Err(format!("environment leaked into status: {text}"));
            }
            stopped
        }
        Err(msg) => Err(msg),
    }
}

async fn start_live(db: &ServerDb) -> Result<Running, String> {
    let running = start_running(db)?;
    let ready = wait_health(db).await;
    match ready {
        Ok(()) => Ok(running),
        Err(msg) => Err(msg),
    }
}

async fn stop_live(mut running: Running, port: u16) -> Result<(), String> {
    let posted = test_harness::test_post(port, "/v1/server/stop", "{}").await;
    let result = finish_running(&mut running).await;
    match posted {
        Ok((_code, _body)) => match result {
            Ok(_outcome) => Ok(()),
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}

async fn finish_running(running: &mut Running) -> Result<Outcome, String> {
    for _tick in 0..80 {
        match running.child().try_wait() {
            Ok(Some(_status)) => return running.finish(),
            Ok(None) => {}
            Err(err) => return Err(err.to_string()),
        }
        test_harness::sleep_real(std::time::Duration::from_millis(250)).await;
    }
    running.kill()
}

#[tokio::test]
async fn health_endpoint_answers_on_the_started_server() -> Result<(), String> {
    let db = db_start("unit_health").await?;
    let mut running = start_running(&db)?;
    let ready = wait_health(&db).await;
    let result = running.kill();
    let stopped = pg_stop(&db.pg).await;
    match result {
        Ok(_outcome) if ready.is_ok() => stopped,
        Ok(_outcome) => ready,
        Err(kill) => Err(kill),
    }
}

#[tokio::test]
async fn second_server_exits_with_one_while_first_keeps_serving_health() -> Result<(), String> {
    let db = db_start("two_servers_second_fails").await?;
    let mut first = start_running(&db)?;
    let ready = wait_health(&db).await;
    let second = run_program(&server_program(db.args()), &db.dir)?;
    let check = check_second(&second, db.listen_port);
    let _result = first.kill();
    let stopped = pg_stop(&db.pg).await;
    match check {
        Ok(()) if ready.is_ok() => stopped,
        Ok(()) => ready,
        Err(msg) => Err(msg),
    }
}

fn check_second(second: &Outcome, port: u16) -> Result<(), String> {
    if second.code != 1 {
        return Err(format!("second server exit {}", second.code));
    }
    if !second.stdout.is_empty() {
        return Err(format!("second server printed startup output {second:?}"));
    }
    let want = format!("Cannot use port {port}:");
    if !second.stderr.contains(&want) {
        return Err(format!("{second:?} misses the port message"));
    }
    Ok(())
}
