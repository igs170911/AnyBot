use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::Response;

use anybot_protocol::{LoginSession, ProblemJson, TwoStepLoginRequest};

use crate::api_account::{Guard, guard_idem};
use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::idem::request_scope;
use crate::req_write::{denied_detail, internal_detail, pool_of, problem_box};
use crate::two_step_challenge::{challenge_fingerprint, challenge_hash};
use crate::two_step_check::secrets_of;
use crate::two_step_login_flow::{StepTwo, replay_passed, run_step_two};

const SEC_FETCH_SITE: &str = "sec-fetch-site";

#[utoipa::path(
    post,
    path = "/v1/account/login/two-step",
    operation_id = "post_account_login_two_step",
    summary = "Finish a login with the two-step code",
    description = "Second step after the password was accepted. Requires the Idempotency-Key header; the record is bound to the source address. The credential kind is the one the first step asked for.",
    tag = "account",
    security(()),
    request_body(content = TwoStepLoginRequest, example = json!({"challenge": "9f2c4e7a1b3d5f60", "code": "123456"})),
    responses(
        (status = 200, description = "Session opened",
            content_type = "application/json",
            body = LoginSession,
            example = json!({"bearer_key": "a1b2c3d4e5f6a1b2", "account": "alice", "device_id": "d1"})),
        (status = 400, description = "Challenge expired or used up",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-expired", "params": {}, "status": 400, "title": "This login has expired; enter your password again.", "type": "https://anybot.test/problems/account_two_step_expired"})),
        (status = 401, description = "Wrong code",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-code-wrong", "params": {}, "status": 401, "title": "The verification code is wrong.", "type": "https://anybot.test/problems/account_two_step_code_wrong"})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-idempotency-conflict", "params": {}, "status": 409, "title": "This request id was already used with a different body.", "type": "https://anybot.test/problems/account_idempotency_conflict"})),
        (status = 429, description = "Too many wrong codes",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-rate", "params": {}, "status": 429, "title": "Too many wrong verification codes; try again in 5 minutes.", "type": "https://anybot.test/problems/account_two_step_rate"})),
        (status = 500, description = "The stored secret cannot be opened",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"code": "account-two-step-unreadable", "params": {}, "status": 500, "title": "The two-step secret of this account cannot be opened on this server.", "type": "https://anybot.test/problems/account_two_step_unreadable"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn login_two_step(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY)
        .await
        .map_err(|e| internal_detail(&e.to_string()))?;
    let key = crate::req_write::idempotency_key_of(&parts.headers)?;
    let address = crate::req_write::parts_address(&parts, &state).await?;
    let origin = crate::api_login_issue::origin_of(&parts.headers);
    let native = origin.is_none() && !parts.headers.contains_key(SEC_FETCH_SITE);
    let body: TwoStepLoginRequest = serde_json::from_slice(&raw)
        .map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let keys = secrets_of(&state)?;
    let id_hash = challenge_hash(&body.challenge);
    let fingerprint = challenge_fingerprint(&id_hash);
    let scope = request_scope(&Method::POST, parts.uri.path(), &address);
    let step = StepTwo {
        state: &state,
        pool,
        keys: &keys,
        address: &address,
        origin: origin.as_deref(),
        native,
        id_hash: &id_hash,
        code: &body.code,
        key: &key,
        fingerprint: &fingerprint,
        scope: &scope,
    };
    match guard_idem(pool, &key, &fingerprint, &scope).await? {
        Guard::Fresh => run_step_two(&step).await,
        Guard::Replay(StatusCode::OK, _, true) => replay_passed(&step).await,
        Guard::Replay(_, value, _) => replayed_problem(value),
    }
}

fn replayed_problem(value: serde_json::Value) -> Result<Response, CallerDenied> {
    let problem: ProblemJson =
        serde_json::from_value(value).map_err(|_| internal_detail("stored problem damaged"))?;
    Ok(problem_box(Box::new(problem)))
}
