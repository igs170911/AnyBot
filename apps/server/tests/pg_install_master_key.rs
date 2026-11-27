use anybot_app_server as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use std::path::Path;

use anybot_app_test::{
    ServerDb, ServerProgram, bootstrap_args, db_start, english, start_live, startup_params,
};
use anybot_messages::{Catalog, MessageCode, Params};
use test_harness::{Outcome, pg_stop, run_program, write_file_with_mode};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

struct BadCase {
    label: &'static str,
    text: String,
    code: MessageCode,
    name: &'static str,
}

fn new_key_line(dir: &Path) -> Result<String, String> {
    let outcome = run_program(
        &server_bin().program(vec!["new-master-key".to_owned()]),
        dir,
    )?;
    if outcome.code != 0 {
        return Err(format!("new-master-key exit {}: {outcome:?}", outcome.code));
    }
    let lines: Vec<&str> = outcome.stdout.lines().collect();
    let [line] = lines.as_slice() else {
        return Err(format!("new-master-key printed {} lines", lines.len()));
    };
    let hex = line
        .strip_prefix("master_key=")
        .ok_or_else(|| "new-master-key line lacks master_key=".to_owned())?;
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "new-master-key value is not 64 hex chars ({})",
            hex.len()
        ));
    }
    Ok((*line).to_owned())
}

fn url_password(url: &str) -> Result<String, String> {
    let (_scheme, rest) = url.split_once("://").ok_or("url without scheme")?;
    let (userinfo, _host) = rest.split_once('@').ok_or("url without userinfo")?;
    let (_user, password) = userinfo.split_once(':').ok_or("url without password")?;
    if password.is_empty() {
        return Err("fixture url has an empty password".to_owned());
    }
    Ok(password.to_owned())
}

fn bad_cases(url: &str, key_line: &str) -> Vec<BadCase> {
    let mut short_key = key_line.to_owned();
    short_key.pop();
    vec![
        BadCase {
            label: "missing_key",
            text: format!("database={url}\n"),
            code: MessageCode::ServerBootstrapNameMissing,
            name: "master_key",
        },
        BadCase {
            label: "unknown_name",
            text: format!("database={url}\n{key_line}\nextra_name=1\n"),
            code: MessageCode::ServerBootstrapNameUnknown,
            name: "extra_name",
        },
        BadCase {
            label: "database_twice",
            text: format!("database={url}\ndatabase={url}\n{key_line}\n"),
            code: MessageCode::ServerBootstrapNameDuplicate,
            name: "database",
        },
        BadCase {
            label: "short_key",
            text: format!("database={url}\n{short_key}\n"),
            code: MessageCode::ServerBootstrapMasterKeyMalformed,
            name: "master_key",
        },
    ]
}

fn check_bad(dir: &Path, case: &BadCase, secrets: &[&str]) -> Result<(), String> {
    let path = dir.join(format!("bad_{}.conf", case.label));
    write_file_with_mode(&path, &case.text, 0o600)?;
    let outcome = run_program(&server_bin().program(bootstrap_args(&path)), dir)?;
    if outcome.code != 2 {
        return Err(format!("{} exit {}", case.label, outcome.code));
    }
    let printed = format!("{}{}", outcome.stdout, outcome.stderr);
    let want = english(
        &mut Catalog::new(),
        case.code,
        &Params::new().with_text("name", case.name),
    )?;
    if !printed.contains(&want) {
        return Err(format!("{} output misses {want}: {printed}", case.label));
    }
    if secrets.iter().any(|secret| printed.contains(secret)) {
        return Err(format!("{} output leaked a secret value", case.label));
    }
    Ok(())
}

async fn good_bootstrap_starts(db: &ServerDb, key_line: &str) -> Result<(), String> {
    write_file_with_mode(
        &db.bootstrap,
        &format!("database={}\n{key_line}\n", db.url),
        0o600,
    )?;
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &startup_params(
            anybot_foundation::log_version(&anybot_foundation::version_string()),
            db.listen_port,
        ),
    )?;
    let mut running = start_live(&server_bin(), db, &expected).await?;
    running.kill().map(|_outcome| ())
}

async fn run_n22_5(db: &ServerDb) -> Result<(), String> {
    let first = new_key_line(&db.dir)?;
    let second = new_key_line(&db.dir)?;
    if first == second {
        return Err("two new-master-key runs printed the same key".to_owned());
    }
    good_bootstrap_starts(db, &first).await?;
    let password = url_password(&db.url)?;
    let key_hex = first.trim_start_matches("master_key=");
    let key_head = key_hex.get(..32).ok_or("key shorter than 32 chars")?;
    for case in bad_cases(&db.url, &first) {
        check_bad(&db.dir, &case, &[password.as_str(), key_head])?;
    }
    Ok(())
}

#[tokio::test]
async fn t_install_n22_5() -> Result<(), String> {
    let db = db_start("install_n22_5").await?;
    let outcome = run_n22_5(&db).await;
    let stopped = pg_stop(&db.pg).await;
    outcome.and(stopped)
}

#[tokio::test]
async fn single_url_line_bootstrap_is_refused() -> Result<(), String> {
    let db = db_start("install_single_line").await?;
    write_file_with_mode(&db.bootstrap, &format!("{}\n", db.url), 0o600)?;
    let outcome = run_program(&server_bin().program(db.args()), &db.dir);
    let stopped = pg_stop(&db.pg).await;
    let outcome: Outcome = outcome?;
    if outcome.code != 2 {
        return Err(format!("single-line bootstrap exit {}", outcome.code));
    }
    let printed = format!("{}{}", outcome.stdout, outcome.stderr);
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerBootstrapLineMalformed,
        &Params::new().with_text("line", "1"),
    )?;
    if !printed.contains(&want) || printed.contains(&url_password(&db.url)?) {
        return Err(format!("single-line output wrong: {printed}"));
    }
    stopped
}
