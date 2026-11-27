use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::computer_actions::AccountAction;
use anybot_messages::MessageCode;
use anybot_protocol::{ActionAck, ComputerDelegateRequest, ComputerRemoveRequest, ProblemJson};
use anybot_store_server::computers;

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computers_action::{Step, finish, prepare, require_allowed};
use crate::computers_service::{find_owned, remove};
use crate::req_write::{CallerDenied, denied, internal_sqlx, pool_of, value_of};

#[utoipa::path(
    post,
    path = "/v1/computers/remove",
    operation_id = "post_computers_remove",
    summary = "Remove a computer from the account",
    description = "A computer may remove itself; removing another one is refused for computers added with a pairing code.",
    tag = "computers",
    security(("bearer" = [])),
    request_body(content = ComputerRemoveRequest, example = json!({"device_id": "018f-device"})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "A paired computer cannot do this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_paired_forbidden", "title": "A computer added with a pairing code cannot do this", "status": 403, "code": "computer-paired-forbidden", "params": {}})),
        (status = 404, description = "No such computer",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_not_found", "title": "There is no such computer in this account", "status": 404, "code": "computer-not-found", "params": {}})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_conflict", "title": "This request id was already used with a different body.", "status": 409, "code": "account-idempotency-conflict", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn remove_computer(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let prepared = match prepare::<ComputerRemoveRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let target = &prepared.body.device_id;
    if *target != caller.device_id {
        require_allowed(&caller, AccountAction::RemoveOtherComputer)?;
    }
    find_owned(pool, &caller.account, target).await?;
    remove(&state, pool, &caller.account, target).await?;
    let value = value_of(&ActionAck { ok: true })?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}

#[utoipa::path(
    post,
    path = "/v1/computers/delegate",
    operation_id = "post_computers_delegate",
    summary = "Switch delegation of the calling computer",
    description = "Only the computer itself may change its own switch.",
    tag = "computers",
    security(("bearer" = [])),
    request_body(content = ComputerDelegateRequest, example = json!({"device_id": "018f-device", "enabled": true})),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = ActionAck,
            example = json!({"ok": true})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 403, description = "A paired computer cannot do this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_paired_forbidden", "title": "A computer added with a pairing code cannot do this", "status": 403, "code": "computer-paired-forbidden", "params": {}})),
        (status = 403, description = "Only the computer itself may change this",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_delegate_self_only", "title": "Only the computer itself can change this switch", "status": 403, "code": "computer-delegate-self-only", "params": {}})),
        (status = 404, description = "No such computer",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_not_found", "title": "There is no such computer in this account", "status": 404, "code": "computer-not-found", "params": {}})),
        (status = 409, description = "Idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_conflict", "title": "This request id was already used with a different body.", "status": 409, "code": "account-idempotency-conflict", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn delegate_computer(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    require_allowed(&caller, AccountAction::ChangeSharing)?;
    let prepared = match prepare::<ComputerDelegateRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let body = &prepared.body;
    if caller.credential != CredentialKind::Computer || body.device_id != caller.device_id {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::ComputerDelegateSelfOnly,
        ));
    }
    find_owned(pool, &caller.account, &body.device_id).await?;
    let changed = computers::set_delegate(pool, &body.device_id, body.enabled)
        .await
        .map_err(internal_sqlx)?;
    if changed {
        crate::computers_log::announce(&state, pool, &caller.account, "delegate", &body.device_id)
            .await?;
    }
    let value = value_of(&ActionAck { ok: true })?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}
