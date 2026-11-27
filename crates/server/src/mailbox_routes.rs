use axum::Router;
use axum::routing::post;

use crate::app::AppState;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/mailbox/round", post(crate::api_mailbox::deliver_round))
        .route("/v1/mailbox/claim", post(crate::api_mailbox::claim_letter))
}
