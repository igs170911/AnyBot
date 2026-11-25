use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tokio::sync::Notify;

use anybot_foundation::{Clock, ServerBootstrap, SystemClock, TimestampMs};
use anybot_store_server::Pool;

use crate::api_account::login;
use crate::api_admin::{invite, rotate_invite, set_public_url};
use crate::api_admin_accounts::{disable_account, enable_account, reset_password};
use crate::api_ops::{change_password, logout, logout_all};
use crate::api_register::register;
use crate::config::ServerConfig;
use crate::health::health;
use crate::pass_hash::PassHash;
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
    pub pass_hash: Arc<tokio::sync::OnceCell<Arc<PassHash>>>,
}

impl AppState {
    pub async fn pass_hasher(&self) -> Result<Arc<PassHash>, String> {
        self.pass_hash
            .get_or_try_init(|| async { PassHash::new().map(Arc::new) })
            .await
            .map(Arc::clone)
    }
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
            pass_hash: Arc::new(tokio::sync::OnceCell::new()),
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
        .route("/v1/account/register", post(register))
        .route("/v1/account/login", post(login))
        .route("/v1/account/logout", post(logout))
        .route("/v1/account/logout-all", post(logout_all))
        .route("/v1/account/password", post(change_password))
        .route("/v1/admin/invite", get(invite))
        .route("/v1/admin/invite/rotate", post(rotate_invite))
        .route("/v1/admin/public-url", post(set_public_url))
        .route(
            "/v1/admin/accounts/{account}/disable",
            post(disable_account),
        )
        .route("/v1/admin/accounts/{account}/enable", post(enable_account))
        .route(
            "/v1/admin/accounts/{account}/reset-password",
            post(reset_password),
        )
        .with_state(state)
}
