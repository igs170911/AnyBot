use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use anybot_server::{
    AppState, ServerConfig, build_router, ensure_schema, open_url, product_migrations,
};

use test_harness::PgInstance;
use test_harness::fake_clock::FakeClock;
use test_harness::fake_model::model_start;
use test_harness::fake_push::push_start;
use test_harness::netctl::net_start;
use test_harness::ports::bind_v4;
use test_harness::postgres::pg_start;
use test_harness::realtime::timeout_real;
use test_harness::scratch::test_dir;

use crate::control::control_router;
use crate::locks::LockTable;
use crate::seed::bootstrap_admin;
use crate::state::{EnvPorts, TestEnvState};
use crate::web_files::serve_web;
use test_harness::server_client::ServerApi;

const START_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);

fn start_server(app: AppState, std_listener: std::net::TcpListener) -> Result<u16, String> {
    let port = std_listener.local_addr().map_err(|e| e.to_string())?.port();
    std_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let listener = TcpListener::from_std(std_listener).map_err(|e| e.to_string())?;
    let app = build_router(app).fallback(serve_web);
    tokio::spawn(async move {
        match axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        {
            Ok(()) => {}
            Err(err) => {
                let mut stderr = std::io::stderr();
                let _result = std::io::Write::write_fmt(
                    &mut stderr,
                    format_args!("testenv server stopped{err}\n"),
                );
            }
        }
    });
    Ok(port)
}

async fn start_database(
    clock: Arc<FakeClock>,
    serve_port: u16,
) -> Result<(AppState, PgInstance), String> {
    let dir = test_dir("testenv")?;
    let pg = timeout_real(pg_start(&dir), START_BUDGET).await??;
    pg.create_database("anybot_test").await?;
    let url = pg.url_for("anybot_test");
    let pool = open_url(&url).await.map_err(|e| e.to_string())?;
    let mut migrations = product_migrations();
    ensure_schema(&pool, &mut migrations)
        .await
        .map_err(|e| format!("testenv migrate rejected: {e}"))?;
    let config = ServerConfig {
        listen_addresses: vec!["127.0.0.1".to_owned()],
        listen_port: serve_port,
        trust_proxy: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        ..ServerConfig::default()
    };
    let master = anybot_foundation::MasterKey::generate().map_err(|e| e.to_string())?;
    let keys = anybot_server::SecretKeys::from_master(&master).map_err(|e| e.to_string())?;
    let state =
        AppState::with_pool("testenv-9.9.9", clock, pool, config).with_secrets(Arc::new(keys));
    Ok((state, pg))
}

pub async fn testenv_start() -> Result<(Arc<TestEnvState>, Router, TcpListener), String> {
    let (net, proxy_port) = timeout_real(net_start(), START_BUDGET).await??;
    let (model, model_port) = timeout_real(model_start(), START_BUDGET).await??;
    let (push, push_port) = timeout_real(push_start(), START_BUDGET).await??;
    let clock = FakeClock::new(1_700_000_000_000);
    let (serve_std, serve_port) = bind_v4()?;
    let (app, pg) = start_database(Arc::clone(&clock), serve_port).await?;
    let pool = app.pool().ok_or("testenv server has no database pool")?;
    let bound_port = start_server(app, serve_std)?;
    let api = ServerApi::new(bound_port)?;
    let admin = timeout_real(bootstrap_admin(&api, &pool), START_BUDGET).await??;
    let (control_std, control_port) = bind_v4()?;
    control_std
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let control_listener = TcpListener::from_std(control_std).map_err(|e| e.to_string())?;
    let state = Arc::new(TestEnvState {
        clock,
        model,
        push,
        net,
        pg,
        api,
        admin,
        locks: LockTable::default(),
        ports: EnvPorts {
            server_port: bound_port,
            model_port,
            push_port,
            proxy_port,
            control_port,
        },
    });
    Ok((
        Arc::clone(&state),
        control_router(Arc::clone(&state)),
        control_listener,
    ))
}
