use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::pairing_rules::WRONG_DELAY_MS;
use anybot_messages::MessageCode;
use anybot_protocol::{LoginSession, PairRedeemRequest, ProblemJson};

use crate::app::AppState;
use crate::limits::Limiter;
use crate::pairing_redeem::{Redeem, redeem_code};
use crate::req_write::{
    CallerDenied, denied, idempotency_key_of, internal_sqlx, json_response, parts_address, pool_of,
    value_of,
};

#[utoipa::path(
    post,
    path = "/v1/computers/pair",
    operation_id = "post_computers_pair",
    summary = "Add a computer with a pairing code",
    description = "No account credential is needed. A wrong code waits 0.3 seconds. The same Idempotency-Key and code from the same address replace the credential of a computer that never used it.",
    tag = "computers",
    request_body(content = PairRedeemRequest, example = json!({"code": "048213", "name": "server room"})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = LoginSession,
            example = json!({"bearer_key": "a1b2c3d4", "account": "alice", "device_id": "018f-device"})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 400, description = "The pairing code is wrong or expired",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computers_pair_wrong", "title": "The pairing code is wrong or expired (valid for 10 minutes, one use)", "status": 400, "code": "computers-pair-wrong", "params": {}})),
        (status = 429, description = "Too many wrong pairing codes",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computers_pair_rate", "title": "Too many wrong pairing codes: try again in 5 minutes", "status": 429, "code": "computers-pair-rate", "params": {}})),
        (status = 400, description = "Name too long",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_name_too_long", "title": "A computer name can have at most 60 characters", "status": 400, "code": "computer-name-too-long", "params": {}})),
        (status = 403, description = "Client version too old",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/install_version_old", "title": "This app version is too old.", "status": 403, "code": "install-version-old", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn redeem_pairing(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY)
        .await
        .map_err(|e| crate::req_write::internal_detail(&e.to_string()))?;
    let key = idempotency_key_of(&parts.headers)?;
    let address = parts_address(&parts, &state).await?;
    let body: PairRedeemRequest = serde_json::from_slice(&raw)
        .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let limiter = Limiter::new(pool, std::sync::Arc::clone(&state.clock));
    let attempt = limiter
        .begin_pairing(&address)
        .await
        .map_err(internal_sqlx)?;
    let Some(attempt) = attempt else {
        wrong_delay(&state).await;
        return Err(denied(
            StatusCode::TOO_MANY_REQUESTS,
            MessageCode::ComputersPairRate,
        ));
    };
    let outcome = redeem_code(
        &state,
        &Redeem {
            key: &key,
            address: &address,
            body: &body,
        },
    )
    .await?;
    let Some(session) = outcome else {
        wrong_delay(&state).await;
        return Err(denied(
            StatusCode::BAD_REQUEST,
            MessageCode::ComputersPairWrong,
        ));
    };
    limiter.cancel_login(attempt).await.map_err(internal_sqlx)?;
    let reply: LoginSession = session;
    Ok(json_response(StatusCode::OK, value_of(&reply)?))
}

async fn wrong_delay(state: &AppState) {
    state
        .clock
        .sleep(std::time::Duration::from_millis(WRONG_DELAY_MS))
        .await;
}
