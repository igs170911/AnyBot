use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use anybot_foundation::{Clock, ServerBootstrap, SystemClock};

use crate::health::health;

#[derive(Clone)]
pub struct AppState {
    pub version: String,
    pub clock: Arc<dyn Clock>,
    pub bootstrap: Arc<Option<ServerBootstrap>>,
}

impl AppState {
    #[must_use]
    pub fn new(version: &str) -> Self {
        AppState::with_clock(version, Arc::new(SystemClock))
    }

    #[must_use]
    pub fn with_clock(version: &str, clock: Arc<dyn Clock>) -> Self {
        AppState {
            version: version.to_owned(),
            clock,
            bootstrap: Arc::new(None),
        }
    }

    #[must_use]
    pub fn with_bootstrap(
        version: &str,
        clock: Arc<dyn Clock>,
        bootstrap: ServerBootstrap,
    ) -> Self {
        AppState {
            version: version.to_owned(),
            clock,
            bootstrap: Arc::new(Some(bootstrap)),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .with_state(state)
}
