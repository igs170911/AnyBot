use axum::Router;
use axum::routing::{get, post};

use crate::app::AppState;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/computers", get(crate::api_computers::list_computers))
        .route(
            "/v1/computers/rename",
            post(crate::api_computers::rename_computer),
        )
        .route(
            "/v1/computers/remove",
            post(crate::api_computers_act::remove_computer),
        )
        .route(
            "/v1/computers/pairing-code",
            post(crate::api_pairing::create_pairing_code),
        )
        .route(
            "/v1/computers/pair",
            post(crate::api_pairing_redeem::redeem_pairing),
        )
        .route(
            "/v1/account/reauth",
            post(crate::api_pairing::reauth_password),
        )
        .route(
            "/v1/computers/delegate",
            post(crate::api_computers_act::delegate_computer),
        )
}
