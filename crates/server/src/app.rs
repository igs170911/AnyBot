use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tokio::sync::Notify;

use anybot_foundation::{Clock, ServerBootstrap, SystemClock, TimestampMs};
use anybot_store_server::Pool;

use crate::config::ServerConfig;
use crate::health::health;
use crate::status::{status, stop};

#[derive(Clone)]
pub struct AppState {
    pub version: String,
    pub clock: Arc<dyn Clock>,
    pub bootstrap: Arc<Option<ServerBootstrap>>,
    pub pool: Arc<Option<Pool>>,
    pub config: Arc<ServerConfig>,
    pub shutdown: Arc<Notify>,
    pub started_at_ms: TimestampMs,
}

impl AppState {
    #[must_use]
    pub fn new(version: &str) -> Self {
        AppState::with_clock(version, Arc::new(SystemClock))
    }

    #[must_use]
    pub fn with_clock(version: &str, clock: Arc<dyn Clock>) -> Self {
        let started_at_ms = clock.now_ms();
        AppState {
            version: version.to_owned(),
            clock,
            bootstrap: Arc::new(None),
            pool: Arc::new(None),
            config: Arc::new(ServerConfig::default()),
            shutdown: Arc::new(Notify::new()),
            started_at_ms,
        }
    }

    #[must_use]
    pub fn with_bootstrap(
        version: &str,
        clock: Arc<dyn Clock>,
        bootstrap: ServerBootstrap,
    ) -> Self {
        let mut state = AppState::with_clock(version, clock);
        state.bootstrap = Arc::new(Some(bootstrap));
        state
    }

    #[must_use]
    pub fn with_pool(
        version: &str,
        clock: Arc<dyn Clock>,
        pool: Pool,
        config: ServerConfig,
    ) -> Self {
        let mut state = AppState::with_clock(version, clock);
        state.pool = Arc::new(Some(pool));
        state.config = Arc::new(config);
        state
    }

    #[must_use]
    pub fn pool(&self) -> Option<Pool> {
        self.pool.as_ref().clone()
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/server/status", get(status))
        .route("/v1/server/stop", post(stop))
        .with_state(state)
}
