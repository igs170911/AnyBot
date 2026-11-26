use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;

use anybot_foundation::{Clock, SystemClock};
use anybot_server::{
    AppState, Migrator, boot_database, build_router, open_url, product_migrations,
};
use test_harness::{Capture, FakeClock};

use crate::ServerDb;
use crate::serverdb::db_start;

pub const TEST_VERSION: &str = "test-0.0.0";

pub struct Served {
    pub db: ServerDb,
    pub clock: Arc<FakeClock>,
    pub pool: anybot_server::Pool,
    pub started: anybot_server::Started,
    pub startup: Arc<Capture>,
    server_shutdown: Arc<tokio::sync::Notify>,
    server_tasks: Vec<tokio::task::JoinHandle<()>>,
    v6_port: u16,
}

pub struct ServedReal {
    pub db: ServerDb,
    pub pool: anybot_server::Pool,
    pub started: anybot_server::Started,
    server_shutdown: Arc<tokio::sync::Notify>,
    server_tasks: Vec<tokio::task::JoinHandle<()>>,
    v6_port: u16,
}

pub async fn serve_start(test: &str) -> Result<Served, String> {
    let db = db_start(test).await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    let clock = FakeClock::new(1_700_000_000_000);
    let clock_dyn: Arc<dyn Clock> = Arc::clone(&clock) as Arc<dyn Clock>;
    let mut migrations: Migrator = product_migrations();
    let started = boot_database(&pool, &*clock_dyn, &mut migrations, TEST_VERSION)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let startup = crate::startup::startup_capture(&started, db.listen_port)?;
    let clock_fast: Arc<FakeClock> = Arc::clone(&clock);
    let clock_arc: Arc<dyn Clock> = clock_fast;
    let config = anybot_server::ServerConfig::from_rows(started.config.rows())?;
    let (server_shutdown, server_tasks, v6_port) =
        bind_fake(&db, clock_arc, pool.clone(), config).await?;
    Ok(Served {
        db,
        clock,
        pool,
        started,
        startup,
        server_shutdown,
        server_tasks,
        v6_port,
    })
}

pub async fn serve_start_real(test: &str) -> Result<ServedReal, String> {
    let db = db_start(test).await?;
    let pool = open_url(&db.url).await.map_err(|e| e.to_string())?;
    let clock = Arc::new(SystemClock);
    let mut migrations: Migrator = product_migrations();
    let started = boot_database(&pool, clock.as_ref(), &mut migrations, TEST_VERSION)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let clock_arc: Arc<dyn Clock> = clock;
    let config = anybot_server::ServerConfig::from_rows(started.config.rows())?;
    let (server_shutdown, server_tasks, v6_port) =
        bind_fake(&db, clock_arc, pool.clone(), config).await?;
    Ok(ServedReal {
        db,
        pool,
        started,
        server_shutdown,
        server_tasks,
        v6_port,
    })
}

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

async fn bind_fake(
    db: &ServerDb,
    clock: Arc<dyn Clock>,
    pool: anybot_server::Pool,
    config: anybot_server::ServerConfig,
) -> Result<
    (
        Arc<tokio::sync::Notify>,
        Vec<tokio::task::JoinHandle<()>>,
        u16,
    ),
    String,
> {
    let mut state = AppState::with_pool(TEST_VERSION, Arc::clone(&clock), pool.clone(), config);
    let listener = bind_with_retry(db.listen_port).await?;
    let v6_listener =
        bind_v6_with_retry(next_port(db.listen_port).ok_or("port overflow".to_owned())?).await?;
    let v6_port = v6_listener.local_addr().map_err(|e| e.to_string())?.port();
    let v4_port = listener.local_addr().map_err(|e| e.to_string())?.port();
    state.listen_ports = Arc::new(vec![v4_port, v6_port]);
    let shutdown = Arc::clone(&state.shutdown);
    let cleanup = anybot_server::cleanup::cleanup_loop(pool, clock, Arc::clone(&shutdown));
    let app = build_router(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
    let app_v6 = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
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
    Ok((shutdown, vec![v4_task, v6_task, cleanup], v6_port))
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

impl Served {
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.db.listen_port)
    }

    #[must_use]
    pub fn base_url_v6(&self) -> String {
        format!("http://[::1]:{}", self.v6_port)
    }

    pub async fn stop_server(&mut self) -> Result<(), String> {
        self.server_shutdown.notify_waiters();
        let running = std::mem::take(&mut self.server_tasks);
        if running.is_empty() {
            return Err("server is not running".to_owned());
        }
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(10))
            .ok_or("deadline overflow".to_owned())?;
        for task in running {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let waited = tokio::time::timeout(left, task)
                .await
                .map_err(|_| "server task did not finish".to_owned())?;
            waited.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn restart_server(&mut self) -> Result<(), String> {
        let rows = anybot_server::read_config_rows(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let config = anybot_server::ServerConfig::from_rows(rows)?;
        let clock_fast: Arc<FakeClock> = Arc::clone(&self.clock);
        let clock_arc: Arc<dyn Clock> = clock_fast;
        let (shutdown, task, v6_port) =
            bind_fake(&self.db, clock_arc, self.pool.clone(), config).await?;
        let _old = std::mem::replace(&mut self.server_shutdown, shutdown);
        self.server_tasks = task;
        self.v6_port = v6_port;
        Ok(())
    }

    pub async fn set_config_and_restart(&mut self, key: &str, value: &str) -> Result<(), String> {
        anybot_server::write_config_row(&self.pool, key, value)
            .await
            .map_err(|e| e.to_string())?;
        self.stop_server().await?;
        self.restart_server().await
    }

    pub async fn stop(mut self) -> Result<(), String> {
        self.stop_server().await?;
        self.db.stop().await
    }
}

impl ServedReal {
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.db.listen_port)
    }

    #[must_use]
    pub fn base_url_v6(&self) -> String {
        format!("http://[::1]:{}", self.v6_port)
    }

    pub async fn stop_server(&mut self) -> Result<(), String> {
        self.server_shutdown.notify_waiters();
        let running = std::mem::take(&mut self.server_tasks);
        if running.is_empty() {
            return Err("server is not running".to_owned());
        }
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(10))
            .ok_or("deadline overflow".to_owned())?;
        for task in running {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            let waited = tokio::time::timeout(left, task)
                .await
                .map_err(|_| "server task did not finish".to_owned())?;
            waited.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn restart_server(&mut self) -> Result<(), String> {
        let rows = anybot_server::read_config_rows(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        let config = anybot_server::ServerConfig::from_rows(rows)?;
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let (shutdown, task, v6_port) =
            bind_fake(&self.db, clock, self.pool.clone(), config).await?;
        let _old = std::mem::replace(&mut self.server_shutdown, shutdown);
        self.server_tasks = task;
        self.v6_port = v6_port;
        Ok(())
    }

    pub async fn set_config_and_restart(&mut self, key: &str, value: &str) -> Result<(), String> {
        anybot_server::write_config_row(&self.pool, key, value)
            .await
            .map_err(|e| e.to_string())?;
        self.stop_server().await?;
        self.restart_server().await
    }

    pub async fn stop(mut self) -> Result<(), String> {
        self.stop_server().await?;
        self.db.stop().await
    }
}

pub async fn boot_pair(served: &Served) -> Result<String, String> {
    let invite = crate::n106::invite_of(&served.started)?;
    crate::n106::register(
        &served.base_url(),
        "alice",
        "alicepassword",
        &invite,
        "k-admin",
    )
    .await?;
    crate::n106::register(&served.base_url(), "bob", "bobpassword", &invite, "k-bob").await?;
    Ok(invite)
}
