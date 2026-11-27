use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tokio::sync::Notify;

use anybot_foundation::{Clock, ServerBootstrap, SystemClock, TimestampMs};
use anybot_store_server::Pool;

use crate::api_account::login;
use crate::api_admin::{invite, rotate_invite, set_public_url};
use crate::api_admin_accounts::{disable_account, enable_account};
use crate::api_admin_reset::reset_password;
use crate::api_ops::change_password;
use crate::api_ops_session::{logout, logout_all};
use crate::api_register::register;
use crate::config::ServerConfig;
use crate::health::health;
use crate::pass_hash::PassHash;
use crate::status::status;

#[derive(Clone)]
pub struct AppState {
    pub version: String,
    pub clock: Arc<dyn Clock>,
    pub bootstrap: Arc<Option<ServerBootstrap>>,
    pub pool: Arc<Option<Pool>>,
    pub config: Arc<tokio::sync::Mutex<ServerConfig>>,
    pub shutdown: Arc<Notify>,
    pub started_at_ms: TimestampMs,
    pub pass_hash: Arc<tokio::sync::OnceCell<Arc<PassHash>>>,
    pub listen_ports: Arc<Vec<u16>>,
    pub releases: Arc<anybot_domain::version_release::ReleaseTable>,
    pub bus: anybot_bus::SharedBus,
    pub secrets: Option<Arc<anybot_store_server::SecretKeys>>,
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
            listen_ports: Arc::new(Vec::new()),
            releases: Arc::new(anybot_domain::version_release::ReleaseTable::empty()),
            clock,
            bootstrap: Arc::new(None),
            pool: Arc::new(None),
            config: Arc::new(tokio::sync::Mutex::new(ServerConfig::default())),
            shutdown: Arc::new(Notify::new()),
            started_at_ms,
            bus: anybot_bus::shared_memory_bus(),
            secrets: None,
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
        state.config = Arc::new(tokio::sync::Mutex::new(config));
        state
    }

    #[must_use]
    pub fn with_releases(mut self, releases: anybot_domain::version_release::ReleaseTable) -> Self {
        self.releases = Arc::new(releases);
        self
    }

    #[must_use]
    pub fn with_secrets(mut self, keys: Arc<anybot_store_server::SecretKeys>) -> Self {
        self.secrets = Some(keys);
        self
    }

    #[must_use]
    pub async fn config_view(&self) -> crate::config::ConfigView {
        self.config.lock().await.view()
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
        .route("/v1/account/register", post(register))
        .route("/v1/account/login", post(login))
        .route("/v1/account/logout", post(logout))
        .route("/v1/account/logout-all", post(logout_all))
        .route("/v1/account/password", post(change_password))
        .route(
            "/v1/streams/append",
            post(crate::api_streams_append::append),
        )
        .route("/v1/streams/after", get(crate::api_streams_after::after))
        .route("/v1/streams/record", get(crate::api_streams_record::record))
        .route("/v1/settings", get(crate::api_settings::settings))
        .route("/v1/streams/referee", post(crate::api_referee::referee))
        .route(
            "/v1/streams/actions",
            post(crate::api_streams_action::action),
        )
        .route(
            "/v1/streams/notifications",
            get(crate::api_streams_sse::notifications),
        )
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
        .merge(two_step_routes())
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth_csrf::csrf_layer,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth_pre::pre_session_layer,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth_version::version_layer,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::auth_time::time_layer,
        ))
}

fn two_step_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/account/login/two-step",
            post(crate::api_two_step_login::login_two_step),
        )
        .route(
            "/v1/account/two-step/setup",
            post(crate::api_two_step_setup::setup_two_step),
        )
        .route(
            "/v1/account/two-step/confirm",
            post(crate::api_two_step_toggle::confirm_two_step),
        )
        .route(
            "/v1/account/two-step/disable",
            post(crate::api_two_step_toggle::disable_two_step),
        )
}
