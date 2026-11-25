use std::net::SocketAddr;
use std::sync::Arc;

use anybot_app_server as _;
use anybot_app_test::{ServerProgram, english};
use anybot_foundation as _;
use anybot_foundation::{Clock, MessageRef, ServerLaunchArgs};
use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_server::{Pool, probe, probe_pool};
use axum as _;
use clap as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tracing as _;

use test_harness::{
    ArcWriter, Capture, FakeClock, NetCtl, ServerDb, db_start, net_attach, net_cut_target,
    net_down, net_probe_open, net_restore, net_start_raw, poll_contains, run_program, test_dir,
    test_post,
};

fn server_bin() -> ServerProgram {
    ServerProgram::new(env!("CARGO_BIN_EXE_anybot-server"))
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
    let db = db_start("server_n8_3_blocked").await?;
    let pg_port = db.pg.port();
    let (ctl, proxy_port) = net_start_raw().await?;
    let real = SocketAddr::from(([127, 0, 0, 1], pg_port));
    let _attached = net_attach(&ctl, proxy_port, real).await;
    let _cut = net_down(&ctl, proxy_port).await;
    eprintln!("n8 t0 down");
    let url = blocked_url(&db, proxy_port);
    let bootstrap = anybot_foundation::bootstrap::parse_connection(&url)
        .map_err(|e| format!("bootstrap parse: {e}"))?;
    let clock = FakeClock::new(1_000);
    let capture = Arc::new(Capture::new());
    let shared = Arc::clone(&capture);
    let plan_clock = Arc::clone(&clock);
    let ticker_clock = Arc::clone(&clock);
    tokio::spawn(async move {
        for _tick in 0..400 {
            let _advanced = ticker_clock.advance(std::time::Duration::from_secs(5));
            test_harness::sleep_real(std::time::Duration::from_millis(25)).await;
        }
    });
    let mut ctx = ctx_with_capture(&shared);
    let cycles = down_probe_cycles(&bootstrap, &mut ctx, &clock, &capture).await;
    if cycles.is_ok_and(|n| n == 0) {
        return Err("no retry lines printed".to_owned());
    }
    let _restored = net_restore(&ctl, proxy_port).await;
    eprintln!("n8 t1 restored");
    let opened = wait_proxy_open(&ctl, proxy_port).await;
    eprintln!("n8 t2 opened");
    let opened = match opened {
        Ok(()) => opened,
        Err(msg) => return Err(msg),
    };
    let joined_plan = test_harness::timeout_real(
        anybot_app_server::server_run::connect_plan(&bootstrap, &mut ctx, plan_clock.as_ref()),
        std::time::Duration::from_secs(12),
    )
    .await?;
    advance_plan_clock(plan_clock.as_ref());
    eprintln!("n8 t3 planned");
    let pool = joined_plan
        .map_err(|e| format!("connect plan stalled: {e}; lines {:?}", capture.lines()))?;
    let online = serve_online(&db, &pool, &capture).await;
    let detached = net_cut_target(&ctl, proxy_port).await;
    eprintln!("n8 t5 detached");
    let stopped = db.stop().await;
    match online {
        Ok(()) => match detached {
            Ok(()) => stopped,
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}

async fn down_probe_cycles(
    bootstrap: &anybot_foundation::ServerBootstrap,
    ctx: &mut anybot_app_server::server_run::Ctx,
    clock: &FakeClock,
    capture: &Arc<Capture>,
) -> Result<usize, String> {
    let mut failures: usize = 0;
    for _round in 0..3 {
        let pool = probe_pool(bootstrap);
        let probed =
            test_harness::timeout_real(probe(&pool), std::time::Duration::from_millis(2200)).await;
        let _closed = pool.close().await;
        if probed.is_err() {
            failures = failures.saturating_add(1);
            let reference = MessageRef::new(anybot_app_server::CODE_SERVER_DB_CONNECT_RETRY)
                .with_text("db_host", &bootstrap.host)
                .with_text("reason", "blocked")
                .with_int("seconds", 5);
            ctx.print_ref(&reference).map_err(|e| e.to_string())?;
            let _advanced = clock.advance(std::time::Duration::from_secs(5));
        }
    }
    let hits = poll_contains(capture, "connect-retry", 3).await;
    match hits {
        Ok(_count) => Ok(failures),
        Err(msg) => Err(msg),
    }
}

fn advance_plan_clock(clock: &FakeClock) {
    let _advanced = clock.advance(std::time::Duration::from_secs(5));
}

fn blocked_url(db: &ServerDb, proxy_port: u16) -> String {
    format!(
        "postgresql://{}:{}@127.0.0.1:{}/anybot_test",
        db.pg.user(),
        db.pg.password().expose(),
        proxy_port
    )
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

async fn serve_online(db: &ServerDb, _pool: &Pool, capture: &Arc<Capture>) -> Result<(), String> {
    let launched = ServerLaunchArgs {
        bootstrap: Some(db.bootstrap.clone()),
        command: None,
    };
    let stored = anybot_server::ServerConfig {
        listen_addresses: vec!["127.0.0.1".to_owned(), "::1".to_owned()],
        listen_port: db.listen_port,
        ..anybot_server::ServerConfig::default()
    };
    anybot_server::set_config_rows(
        _pool,
        &stored.rows(),
        &[
            anybot_server::KEY_LISTEN_ADDRESSES,
            anybot_server::KEY_LISTEN_PORT,
        ],
    )
    .await
    .map_err(|e| e.to_string())?;
    let mut ctx = ctx_with_capture(capture);
    let running = test_harness::timeout_real(
        anybot_app_server::server_run::serve_main(&launched, env!("CARGO_PKG_VERSION"), &mut ctx),
        std::time::Duration::from_secs(40),
    );
    let checked = startup_line_check(capture, db).await;
    let stopped_http = test_post(db.listen_port, "/v1/stop", "{}").await;
    let joined = running.await;
    eprintln!(
        "n8 debug joined={joined:?} stopped={stopped_http:?} lines={:?}",
        capture.lines()
    );
    match checked {
        Ok(()) => match stopped_http {
            Ok((200, _body)) => match joined {
                Ok(_code) => Ok(()),
                Err(_slow) => Err("serve_main did not finish".to_owned()),
            },
            Ok((code, _body)) => Err(format!("stop answered {code}")),
            Err(msg) => Err(msg),
        },
        Err(msg) => Err(msg),
    }
}
async fn startup_line_check(capture: &Arc<Capture>, db: &ServerDb) -> Result<(), String> {
    let want = english(
        &mut Catalog::new(),
        MessageCode::ServerStartupListening,
        &Params::new()
            .with_text("version", env!("CARGO_PKG_VERSION"))
            .with_text(
                "addresses",
                &format!("127.0.0.1:{}, [::1]:{}", db.listen_port, db.listen_port),
            )
            .with_text("db_host", "127.0.0.1")
            .with_text("db_name", "anybot_test"),
    )?;
    for _tick in 0..120 {
        let found = capture.lines().iter().any(|l| l.trim() == want);
        if found {
            return Ok(());
        }
        test_harness::sleep_real(std::time::Duration::from_millis(100)).await;
    }
    Err(format!(
        "startup line {want} missing; lines {:?}",
        capture.lines()
    ))
}

async fn wait_proxy_open(ctl: &Arc<NetCtl>, proxy_port: u16) -> Result<(), String> {
    for _tick in 0..300 {
        let open = net_probe_open(ctl, proxy_port).await?;
        if open {
            return Ok(());
        }
        test_harness::sleep_real(std::time::Duration::from_millis(200)).await;
    }
    Err("proxy never reopened".to_owned())
}
