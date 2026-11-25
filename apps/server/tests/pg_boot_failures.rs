use std::net::SocketAddr;
use std::sync::Arc;

use anybot_app_server as _;
use anybot_foundation as _;
use anybot_foundation::SystemClock;
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_server::{
    DEFAULT_PORT, Pool, ServerConfig, boot, open_url, product_migrations, read_config_rows,
    set_config_rows, write_config_row,
};
use axum as _;
use clap as _;
use serde_json as _;
use sqlx::Row as _;

use test_harness::{
    NetCtl, Program, Running, ServerDb, db_start, migration_fixture, net_attach, net_down,
    net_restore, net_start_raw, pg_start, pg_stop, run_program, test_dir,
};

fn server_program(args: Vec<String>) -> Program {
    Program::new(env!("CARGO_BIN_EXE_anybot-server"), args)
}

fn english(catalog: &mut Catalog, code: MessageCode, params: &Params) -> Result<String, String> {
    catalog
        .format(Lang::En, &code, params)
        .map_err(|e| e.to_string())
}

fn product_migrations_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations/server")
}

async fn fixture_migrator(extra: &str) -> Result<anybot_server::Migrator, String> {
    let dir = migration_fixture(&product_migrations_dir(), extra, extra)?;
    anybot_server::Migrator::new(dir)
        .await
        .map_err(|e| e.to_string())
}

async fn open_pool(db: &ServerDb) -> Result<Pool, String> {
    open_url(&db.url).await.map_err(|e| e.to_string())
}

async fn public_table_names(pool: &Pool) -> Result<Vec<String>, String> {
    let rows = sqlx::query(
        "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname = 'public' ORDER BY tablename",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for row in rows {
        names.push(
            row.try_get::<String, _>("tablename")
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(names)
}

#[tokio::test]
async fn t_server_n8_3() -> Result<(), String> {
    let first = run_against_foreign_database().await;
    let second = run_after_database_comes_online().await;
    match first {
        Ok(()) => second,
        Err(msg) => Err(msg),
    }
}

async fn run_against_foreign_database() -> Result<(), String> {
    let dir = test_dir("server_n8_3_foreign").unwrap_or(std::path::PathBuf::from("."));
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let url = pg.url_for("anybot_test");
    let pool = open_url(&url).await.map_err(|e| e.to_string())?;
    sqlx::query("CREATE TABLE gadget (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let bootstrap = dir.join("boot.conf");
    let _written =
        test_harness::scratch::write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600);
    let outcome = run_program(
        &server_program(vec![
            "--bootstrap".to_owned(),
            bootstrap.display().to_string(),
        ]),
        &dir,
    )?;
    let check = foreign_check(&outcome);
    let tables = public_table_names(&pool).await;
    let result = pg_stop(&pg).await;
    match check {
        Ok(()) => match tables {
            Ok(names) => {
                if names.first().is_none_or(|n| n != "gadget") {
                    return Err(format!("foreign table vanished: {names:?}"));
                }
                result
            }
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}

fn foreign_check(outcome: &test_harness::Outcome) -> Result<(), String> {
    if outcome.code != 2 {
        return Err(format!("foreign-data exit {}", outcome.code));
    }
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerDbForeignData,
        &Params::new()
            .with_text("db_host", "127.0.0.1")
            .with_text("db_name", "anybot_test"),
    )?;
    if !outcome.stdout.contains(&expected) {
        return Err(format!("{outcome:?} misses {expected}"));
    }
    Ok(())
}

async fn run_after_database_comes_online() -> Result<(), String> {
    let dir = test_dir("server_n8_3_blocked").unwrap_or(std::path::PathBuf::from("."));
    let pg = pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let (ctl, proxy_port) = net_start_raw().await?;
    let real = SocketAddr::from(([127, 0, 0, 1], pg.port()));
    let _attached = net_attach(&ctl, proxy_port, real).await;
    let _cut = net_down(&ctl, proxy_port).await;
    let url = format!(
        "postgresql://{}:{}@127.0.0.1:{}/anybot_test",
        pg.user(),
        pg.password().expose(),
        proxy_port
    );
    let bootstrap = dir.join("boot.conf");
    let _written =
        test_harness::scratch::write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600);
    let user = pg.user();
    let pass = pg.password().expose().to_owned();
    let started =
        start_blocked_running(&ctl, proxy_port, &dir, &bootstrap, &user, &pass, pg.port()).await;
    let stopped = pg_stop(&pg).await;
    match started {
        Ok(()) => stopped,
        Err(msg) => Err(msg),
    }
}

async fn start_blocked_running(
    ctl: &Arc<NetCtl>,
    proxy_port: u16,
    dir: &std::path::Path,
    bootstrap: &std::path::Path,
    pg_user: &str,
    pg_pass: &str,
    pg_port: u16,
) -> Result<(), String> {
    let mut running = Running::start(
        &server_program(vec![
            "--bootstrap".to_owned(),
            bootstrap.display().to_string(),
        ]),
        dir,
    )?;
    let want = "retrying in 5 seconds".to_owned();
    let first = running.next_stdout_line()?;
    if !first.contains(&want) {
        return Err(format!("blocked server printed {first}"));
    }
    test_harness::sleep_real(std::time::Duration::from_secs(20)).await;
    let _restored = net_restore(ctl, proxy_port).await;
    let direct = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_lazy(&format!(
            "postgres://{}:{}@127.0.0.1:{}/anybot_test",
            pg_user, pg_pass, proxy_port
        ));
    let one = match direct {
        Ok(pool) => sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(format!("direct open {e}")),
    };
    let _one = one.map_err(|e| format!("probe after restore {e}"))?;
    let mut seen = Vec::new();
    for _round in 0..8 {
        let line = running.next_stdout_line()?;
        if line.contains("listening on") {
            let _result = running.kill();
            return Ok(());
        }
        if !line.contains(&want) {
            let _result = running.kill();
            return Err(format!("unexpected line {line}"));
        }
        seen.push(line);
    }
    let gone = running.finish();
    let note = running.stderr_note();
    let last = seen.last().map(|s| s.trim().to_owned()).unwrap_or_default();
    let _unused = (pg_port, proxy_port);
    return Err(format!(
        "no listening line; last {last}; gone {gone:?}; note {note}; pg {pg_port} proxy {proxy_port}"
    ));
    let _result = running.kill();
    Err("server never came online after restore".to_owned())
}

#[tokio::test]
async fn t_server_boot_3_migrations_go_up_one_version_and_back() -> Result<(), String> {
    let db = db_start("server_boot_3").await?;
    let pool = open_pool(&db).await?;
    let clock = Arc::new(SystemClock);
    let first = {
        let mut m = product_migrations();
        boot(&pool, clock.as_ref(), &mut m, "9.9.9").await
    };
    let upgraded = fixture_migrator("0002_fixture_extra.sql").await;
    let outcome = match first {
        Ok(_started) => match upgraded {
            Ok(mut migrator) => boot_up_and_down(&pool, clock.as_ref(), &mut migrator).await,
            Err(msg) => Err(msg),
        },
        Err(err) => Err(format!("product boot rejected: {err:?}")),
    };
    let _result = db.stop().await;
    outcome
}

async fn boot_up_and_down(
    pool: &Pool,
    clock: &SystemClock,
    upgraded: &mut anybot_server::Migrator,
) -> Result<(), String> {
    let up = boot(pool, clock, upgraded, "9.9.10").await;
    match up {
        Ok(_started) => {}
        Err(err) => {
            return Err(format!("upgraded boot rejected: {err:?}"));
        }
    }
    let names = public_table_names(pool).await?;
    if !names.iter().any(|n| n == "fixture_extra") {
        return Err(format!("upgrade added no table: {names:?}"));
    }
    let back = {
        let mut m = product_migrations();
        boot(pool, clock, &mut m, "9.9.9").await
    };
    match back {
        Ok(_started) => {}
        Err(err) => {
            return Err(format!("downgraded boot rejected: {err:?}"));
        }
    }
    let kept = public_table_names(pool).await?;
    if !kept.iter().any(|n| n == "fixture_extra") {
        return Err(format!("downgrade lost data: {kept:?}"));
    }
    Ok(())
}

#[tokio::test]
async fn migration_failure_keeps_the_schema_and_the_old_set_boots() -> Result<(), String> {
    let db = db_start("boot_2_failure").await?;
    let pool = open_pool(&db).await?;
    let clock = Arc::new(SystemClock);
    let before = public_table_names(&pool).await?;
    let broken = fixture_migrator("0002_fixture_broken.sql").await;
    let outcome = run_broken_migration(&pool, clock.as_ref(), broken, &before).await;
    let old = {
        let mut m = product_migrations();
        boot(&pool, clock.as_ref(), &mut m, "9.9.9").await
    };
    let _result = db.stop().await;
    match old {
        Ok(_started) => outcome,
        Err(err) => Err(format!("old set rejected after failure: {err:?}")),
    }
}

async fn run_broken_migration(
    pool: &Pool,
    clock: &SystemClock,
    broken: Result<anybot_server::Migrator, String>,
    before: &[String],
) -> Result<(), String> {
    let mut migrator = match broken {
        Ok(m) => m,
        Err(msg) => return Err(msg),
    };
    let result = boot(pool, clock, &mut migrator, "9.9.10").await;
    if result.is_ok() {
        return Err("broken migration was accepted".to_owned());
    }
    let after = public_table_names(pool).await?;
    if after != before {
        return Err(format!("schema changed after failure: {after:?}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_account_bind_4_invalid_listen_address_is_rejected_at_startup() -> Result<(), String> {
    let db = db_start("account_bind_4").await?;
    let pool = open_pool(&db).await?;
    set_config_rows(
        &pool,
        &[("listen_addresses".to_owned(), "abc".to_owned())],
        &[],
    )
    .await
    .map_err(|e| e.to_string())?;
    let outcome = run_program(&server_program(db.args()), &db.dir)?;
    let _result = pg_stop(&db.pg).await;
    let code = i64::from(outcome.code);
    if code != 1 {
        return Err(format!("invalid-address exit {}", outcome.code));
    }
    let expected = english(
        &mut Catalog::new(),
        MessageCode::ServerListenAddressInvalid,
        &Params::new()
            .with_text("address", "abc")
            .with_text("reason", "not an IP address"),
    )?;
    if !outcome.stdout.contains(&expected) {
        return Err(format!("{outcome:?} misses {expected}"));
    }
    if outcome.stdout.contains("listening on") {
        return Err("server announced listening despite the bad address".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn startup_logs_announce_no_accounts_and_a_new_invite() -> Result<(), String> {
    let db = db_start("startup_logs").await?;
    let program = server_program(db.args()).with_timeout(std::time::Duration::from_secs(40));
    let mut running = Running::start(&program, &db.dir)?;
    let _announced = running.next_stdout_line()?;
    let no_accounts = running.next_stdout_line()?;
    let invite = running.next_stdout_line()?;
    let result = running.kill();
    let _stopped = pg_stop(&db.pg).await;
    check_startup_logs(&no_accounts, &invite, result)
}

fn check_startup_logs(
    no_accounts: &str,
    invite: &str,
    result: Result<test_harness::Outcome, String>,
) -> Result<(), String> {
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupNoAccounts,
        &Params::new(),
    )?;
    if no_accounts.trim() != want {
        return Err(format!(
            "no-accounts line {no_accounts} differs from {want}"
        ));
    }
    let code = invite_code_from_line(invite)?;
    if invite_code_shape(&code).is_err() {
        return Err(format!("bad generated invite {code}"));
    }
    match result {
        Ok(_outcome) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn invite_code_from_line(line: &str) -> Result<String, String> {
    let parts: Vec<&str> = line.split("Generated invite code ").collect();
    if parts.len() != 2 {
        return Err(format!("invite line {line} has no code"));
    }
    let code = parts
        .get(1)
        .ok_or("no invite code part")?
        .split(';')
        .next()
        .unwrap_or_default()
        .to_owned();
    Ok(code)
}

fn invite_code_shape(code: &str) -> Result<(), String> {
    let parts: Vec<&str> = code.split('-').collect();
    let head = parts.first().ok_or("no invite head")?;
    let tail = parts.get(1).ok_or("no invite tail")?;
    if parts.len() != 2 || head.len() != 4 || tail.len() != 4 {
        return Err(format!("bad invite shape {code}"));
    }
    for ch in code.chars() {
        if ch == '-' {
            continue;
        }
        if !ch.is_ascii_alphanumeric() || "01lOI".contains(ch) {
            return Err(format!("bad invite character {ch} in {code}"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn startup_listens_on_the_addresses_stored_in_the_database() -> Result<(), String> {
    let db = db_start("account_bind_1").await?;
    let mut running = start_and_hold(&db)?;
    let v4 = health_twice(db.listen_port, "127.0.0.1").await;
    let v6 = health_twice(db.listen_port, "[::1]").await;
    let third = test_harness::http_get_addr(&format!("127.0.0.2:{}", db.listen_port), "/v1/health")
        .await
        .is_ok();
    let result = running.kill();
    let _stopped = pg_stop(&db.pg).await;
    match health_shapes(v4, v6, third, result) {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg),
    }
}

fn start_and_hold(db: &ServerDb) -> Result<Running, String> {
    let mut running = Running::start(&server_program(db.args()), &db.dir)?;
    let announced = running.next_stdout_line()?;
    if !announced.contains("listening on") {
        let _result = running.kill();
        return Err(format!("startup line {announced} is not a listening line"));
    }
    Ok(running)
}

async fn health_twice(port: u16, host: &str) -> Result<(u16, String), String> {
    for _attempt in 0..40 {
        match test_harness::http_get_addr(&format!("{host}:{port}"), "/v1/health").await {
            Ok(out) => return Ok(out),
            Err(_e) => test_harness::sleep_real(std::time::Duration::from_millis(250)).await,
        }
    }
    Err(format!("no answer from {host}:{port}"))
}

fn health_shapes(
    v4: Result<(u16, String), String>,
    v6: Result<(u16, String), String>,
    third_ok: bool,
    result: Result<test_harness::Outcome, String>,
) -> Result<(), String> {
    match v4 {
        Ok((200, _)) => {}
        other => return Err(format!("loopback v4 health {other:?}")),
    }
    match v6 {
        Ok((200, _)) => {}
        other => return Err(format!("loopback v6 health {other:?}")),
    }
    if third_ok {
        return Err("a non-listening loopback address answered".to_owned());
    }
    match result {
        Ok(_outcome) => Ok(()),
        Err(msg) => Err(msg),
    }
}

#[tokio::test]
async fn config_defaults_and_public_url_round_trip_through_the_database() -> Result<(), String> {
    let db = db_start("config_rows").await?;
    let pool = open_pool(&db).await?;
    let rows = read_config_rows(&pool).await.map_err(|e| e.to_string())?;
    let config = ServerConfig::from_rows(rows.clone());
    let _written = write_config_row(&pool, "public_url", "https://round.example/")
        .await
        .map_err(|e| e.to_string());
    let after = read_config_rows(&pool).await.map_err(|e| e.to_string())?;
    let _result = db.stop().await;
    check_config_rows(&config, &rows, &after)
}

fn check_config_rows(
    config: &ServerConfig,
    rows: &[(String, String)],
    after: &[(String, String)],
) -> Result<(), String> {
    if config.listen_port == DEFAULT_PORT && !rows.is_empty() {
        return Err(format!("fixture rows missing: {rows:?}"));
    }
    if config.listen_port == DEFAULT_PORT {
        return Err("fixture port equals the default".to_owned());
    }
    if !after
        .iter()
        .any(|(k, v)| k == "public_url" && v == "https://round.example/")
    {
        return Err(format!("public_url not stored: {after:?}"));
    }
    Ok(())
}
