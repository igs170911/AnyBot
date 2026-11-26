use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;

use anybot_app_server as _;
use anybot_app_test::{ServerDb, ServerProgram, db_start, english};
use anybot_foundation as _;
use anybot_foundation::ServerLaunchArgs;
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use test_harness::{ArcWriter, Capture, FakeClock, run_program, test_dir};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
}

#[tokio::test]
async fn foreign_database_exits_and_a_blocked_database_retries_on_a_fake_clock()
-> Result<(), String> {
    let first = run_against_foreign_database().await;
    let second = run_after_database_comes_online().await;
    match first {
        Ok(()) => second,
        Err(msg) => Err(msg),
    }
}

async fn run_against_foreign_database() -> Result<(), String> {
    let dir = test_dir("server_n8_3_foreign")?;
    let pg = test_harness::pg_start(&dir).await?;
    let _created = pg.create_database("anybot_test").await;
    let url = pg.url_for("anybot_test");
    let pool = anybot_server::open_url(&url)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("CREATE TABLE gadget (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let bootstrap = dir.join("boot.conf");
    let _written =
        test_harness::scratch::write_file_with_mode(&bootstrap, &format!("{url}\n"), 0o600);
    let before = anybot_app_test::public_table_names(&pool).await?;
    let outcome = run_program(
        &server_bin().program(vec![
            "--bootstrap".to_owned(),
            bootstrap.display().to_string(),
        ]),
        &dir,
    )?;
    let check = foreign_check(&outcome);
    let tables = anybot_app_test::public_table_names(&pool).await;
    let result = test_harness::pg_stop(&pg).await;
    match check {
        Ok(()) => match tables {
            Ok(names) => {
                if names != before {
                    return Err(format!(
                        "foreign tables changed: before {before:?} after {names:?}"
                    ));
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
    let mut db = db_start("server_n8_3_blocked").await?;
    let _paused = db.pg.pause().await;
    let bootstrap = anybot_foundation::bootstrap::parse_connection(&db.url)
        .map_err(|e| format!("bootstrap parse: {e}"))?;
    let clock = FakeClock::new(0);
    let capture = Arc::new(Capture::new());
    let mut ctx = ctx_with_capture(&capture);
    let mut plan = Box::pin(anybot_app_server::server_run::connect_plan(
        &bootstrap,
        &mut ctx,
        clock.as_ref(),
    ));
    let walk = walk_to_online(plan.as_mut(), &capture, &clock, &mut db).await;
    let pool = match walk {
        Ok(pool) => pool,
        Err(msg) => {
            let _stopped = db.stop().await;
            return Err(msg);
        }
    };
    let _closed = pool.close().await;
    let online = serve_online(&db, &capture).await;
    let stopped = db.stop().await;
    match online {
        Ok(()) => stopped,
        Err(msg) => Err(msg),
    }
}

async fn walk_to_online<F>(
    mut plan: std::pin::Pin<&mut F>,
    capture: &Arc<Capture>,
    clock: &Arc<FakeClock>,
    db: &mut ServerDb,
) -> Result<anybot_server::Pool, String>
where
    F: Future<Output = Result<anybot_server::Pool, String>>,
{
    let mut round: u64 = 0;
    for _tick in 0..600 {
        tokio::select! {
            joined = &mut plan => {
                return joined.map_err(|e| format!("connect plan stalled: {e}; lines {:?}", capture.lines()));
            }
            () = test_harness::sleep_real(std::time::Duration::from_millis(50)) => {
                let hits = count_retry_lines(capture) as u64;
                if hits <= round {
                    continue;
                }
                round = hits;
                let want_ms = round.saturating_sub(1).saturating_mul(5_000);
                if clock.now_ms() != want_ms {
                    return Err(format!(
                        "retry {round} at fake {} ms, want {want_ms}",
                        clock.now_ms()
                    ));
                }
                if round >= 5 {
                    db.pg.resume().await?;
                }
                let _advanced = clock.advance(std::time::Duration::from_secs(5));
            }
        }
    }
    Err(format!(
        "connect plan budget exceeded; round {round}, lines {:?}",
        capture.lines()
    ))
}

fn count_retry_lines(capture: &Arc<Capture>) -> usize {
    capture
        .lines()
        .iter()
        .filter(|l| l.contains("retrying in 5 seconds"))
        .count()
}

fn ctx_with_capture(sink: &Arc<Capture>) -> anybot_app_server::server_run::Ctx {
    anybot_app_server::server_run::Ctx {
        catalog: Catalog::new(),
        lang: Lang::En,
        stdout: Box::new(ArcWriter {
            sink: Arc::clone(sink),
        }),
        stderr: Box::new(ArcWriter {
            sink: Arc::clone(sink),
        }),
    }
}

async fn serve_online(db: &ServerDb, capture: &Arc<Capture>) -> Result<(), String> {
    let launched = ServerLaunchArgs {
        bootstrap: Some(db.bootstrap.clone()),
        command: None,
    };
    let stored = anybot_server::ServerConfig {
        listen_addresses: vec!["127.0.0.1".to_owned(), "::1".to_owned()],
        listen_port: db.listen_port,
        ..anybot_server::ServerConfig::default()
    };
    let pool = anybot_server::open_url(&db.url)
        .await
        .map_err(|e| e.to_string())?;
    anybot_server::set_config_rows(
        &pool,
        &stored.rows(),
        &[
            anybot_server::KEY_LISTEN_ADDRESSES,
            anybot_server::KEY_LISTEN_PORT,
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    let _closed = pool.close().await;
    let want = startup_want(db)?;
    let mut ctx = ctx_with_capture(capture);
    let staged =
        anybot_app_server::server_run::stage_serve(
            &launched,
            anybot_foundation::log_version(&anybot_foundation::version_string()),
            &mut ctx,
        )
            .await;
    let staged = match staged {
        Ok(staged) => staged,
        Err(code) => return Err(format!("stage failed {code:?}")),
    };
    let stop_handle = Arc::clone(&staged.shutdown);
    let mut serve = Box::pin(anybot_app_server::server_run::finish_serve(
        staged, &mut ctx,
    ));
    let watched = watch_startup_line(serve.as_mut(), capture, &want).await;
    let _notified = stop_handle.notified();
    stop_handle.notify_waiters();
    let joined = test_harness::timeout_real(serve, std::time::Duration::from_secs(20)).await;
    match watched {
        Ok(()) => match joined {
            Ok(_code) => Ok(()),
            Err(_slow) => Err("serve_main did not finish".to_owned()),
        },
        Err(msg) => Err(msg),
    }
}

fn startup_want(db: &ServerDb) -> Result<String, String> {
    english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &Params::new()
            .with_text(
                "version",
                anybot_foundation::log_version(&anybot_foundation::version_string()),
            )
            .with_text(
                "addresses",
                &format!("127.0.0.1:{}, [::1]:{}", db.listen_port, db.listen_port),
            )
            .with_text("db_host", "127.0.0.1")
            .with_text("db_name", "anybot_test"),
    )
}

async fn watch_startup_line<F>(
    mut serve: std::pin::Pin<&mut F>,
    capture: &Arc<Capture>,
    want: &str,
) -> Result<(), String>
where
    F: Future<Output = ExitCode>,
{
    for _tick in 0..400 {
        tokio::select! {
            code = &mut serve => {
                return Err(format!(
                    "serve_main exited early {code:?}; lines {:?}",
                    capture.lines()
                ));
            }
            () = test_harness::sleep_real(std::time::Duration::from_millis(50)) => {
                if capture.lines().iter().any(|l| l.trim() == want) {
                    return Ok(());
                }
            }
        }
    }
    Err(format!(
        "startup line {want} missing; lines {:?}",
        capture.lines()
    ))
}
