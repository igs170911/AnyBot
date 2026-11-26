use std::sync::Arc;

use tokio::net::TcpListener;

use anybot_foundation::Clock;
use anybot_server::ServerConfig;
use test_harness::{Capture, FakeClock};

use crate::ServerDb;

mod serve_lifecycle;
mod serve_start;

pub const TEST_VERSION: &str = "test-0.0.0";

pub struct Served {
    pub db: ServerDb,
    pub clock: Arc<FakeClock>,
    pub pool: anybot_server::Pool,
    pub started: anybot_server::Started,
    pub startup: Arc<Capture>,
    pub state: anybot_server::AppState,
    server_shutdown: Arc<tokio::sync::Notify>,
    server_tasks: Vec<tokio::task::JoinHandle<()>>,
    v6_port: u16,
}

pub struct ServedReal {
    pub db: ServerDb,
    pub pool: anybot_server::Pool,
    pub started: anybot_server::Started,
    pub state: anybot_server::AppState,
    server_shutdown: Arc<tokio::sync::Notify>,
    server_tasks: Vec<tokio::task::JoinHandle<()>>,
    v6_port: u16,
}

pub use serve_lifecycle::boot_pair;
pub use serve_start::{serve_start, serve_start_real, serve_start_with};

#[must_use]
pub fn next_port(port: u16) -> Option<u16> {
    port.checked_add(1)
}

async fn bind_with_retry(port: u16) -> Result<TcpListener, String> {
    let mut last_error = String::new();
    for _attempt in 0..40 {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => return Ok(listener),
            Err(e) => {
                last_error = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
    Err(last_error)
}

async fn bind_v6_with_retry(port: u16) -> Result<TcpListener, String> {
    let mut last_error = String::new();
    for _attempt in 0..40 {
        match TcpListener::bind(("::1", port)).await {
            Ok(listener) => return Ok(listener),
            Err(e) => {
                last_error = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
    Err(last_error)
}

async fn bind_fake(
    db: &ServerDb,
    clock: Arc<dyn Clock>,
    pool: anybot_server::Pool,
    config: ServerConfig,
    releases: &anybot_domain::version_release::ReleaseTable,
) -> Result<
    (
        Arc<tokio::sync::Notify>,
        Vec<tokio::task::JoinHandle<()>>,
        u16,
        anybot_server::AppState,
    ),
    String,
> {
    let mut state =
        anybot_server::AppState::with_pool(TEST_VERSION, Arc::clone(&clock), pool.clone(), config);
    state = state.with_releases(releases.clone());
    let listener = bind_with_retry(db.listen_port).await?;
    let v6_listener =
        bind_v6_with_retry(next_port(db.listen_port).ok_or("port overflow".to_owned())?).await?;
    let v6_port = v6_listener.local_addr().map_err(|e| e.to_string())?.port();
    let v4_port = listener.local_addr().map_err(|e| e.to_string())?.port();
    state.listen_ports = Arc::new(vec![v4_port, v6_port]);
    let shutdown = Arc::clone(&state.shutdown);
    let cleanup = anybot_server::cleanup::cleanup_loop(pool, clock, Arc::clone(&shutdown));
    let app = anybot_server::build_router(state.clone())
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    let app_v6 = anybot_server::build_router(state.clone())
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    let spawned_shutdown = Arc::clone(&shutdown);
    let v4_task = tokio::spawn(async move {
        tokio::select! {
            serve = axum::serve(listener, app) => {
                drop(serve);
            }
            _stop = spawned_shutdown.notified() => {}

        }
    });
    let spawned_shutdown_v6 = Arc::clone(&shutdown);
    let v6_task = tokio::spawn(async move {
        tokio::select! {
            serve = axum::serve(v6_listener, app_v6) => {
                drop(serve);
            }
            _stop = spawned_shutdown_v6.notified() => {}
        }
    });
    Ok((shutdown, vec![v4_task, v6_task, cleanup], v6_port, state))
}
