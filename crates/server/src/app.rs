use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use anybot_foundation::{Clock, SystemClock};

use crate::health::health;

#[derive(Clone)]
pub struct AppState {
    pub version: String,
    pub clock: Arc<dyn Clock>,
}

impl AppState {
    #[must_use]
    pub fn new(version: &str) -> Self {
        AppState {
            version: version.to_owned(),
            clock: Arc::new(SystemClock),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .with_state(state)
}
