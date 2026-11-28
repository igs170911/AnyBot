use axum::Router;
use axum::routing::{delete, get, post};

use crate::app::AppState;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/push/server-key", get(crate::api_push::server_key))
        .route("/v1/push/subscriptions", post(crate::api_push::subscribe))
        .route("/v1/push/subscriptions/mine", get(crate::api_push::mine))
        .route(
            "/v1/push/subscriptions/{id}",
            delete(crate::api_push::unsubscribe),
        )
}
