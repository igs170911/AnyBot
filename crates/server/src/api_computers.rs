use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::computer_actions::AccountAction;
use anybot_protocol::{ActionAck, ComputerRenameRequest, ComputersView, ProblemJson};

use crate::app::AppState;
use crate::caller::Caller;
use crate::computers_action::{Step, finish, prepare, require_allowed};
use crate::computers_service::{list_view, rename};
use crate::req_write::{CallerDenied, json_response, pool_of, value_of};

#[utoipa::path(
    get,
    path = "/v1/computers",
    operation_id = "get_computers",
    summary = "List the computers of the account",
    description = "Every live computer with its online state, and when and why it went offline.",
    tag = "computers",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Done",
            content_type = "application/json",
            body = ComputersView,
            example = json!({"computers": [{"device_id": "018f-device", "name": "study", "online": true, "delegate_enabled": false, "paired": false}]})),
        (status = 401, description = "Credential rejected",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_login_credentials", "title": "This session is no longer valid.", "status": 401, "code": "account-login-credentials", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
)]
pub async fn list_computers(
    State(state): State<AppState>,
    caller: Caller,
) -> Result<Response, CallerDenied> {
    let pool = pool_of(&state)?;
    let view: ComputersView = list_view(&state, pool, &caller.account).await?;
    Ok(json_response(StatusCode::OK, value_of(&view)?))
}

#[utoipa::path(
    post,
    path = "/v1/computers/rename",
    operation_id = "post_computers_rename",
    summary = "Rename a computer",
    description = "Control characters become spaces; at most 60 characters; the name must be free in the account.",
    tag = "computers",
    security(("bearer" = [])),
    request_body(content = ComputerRenameRequest, example = json!({"device_id": "018f-device", "name": "study"})),
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
        (status = 404, description = "No such computer",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_not_found", "title": "There is no such computer in this account", "status": 404, "code": "computer-not-found", "params": {}})),
        (status = 400, description = "Name too long",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_name_too_long", "title": "A computer name can have at most 60 characters", "status": 400, "code": "computer-name-too-long", "params": {}})),
        (status = 409, description = "Name already used",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/computer_name_taken", "title": "The account already has a computer called study", "status": 409, "code": "computer-name-taken", "params": {}})),
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
pub async fn rename_computer(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    require_allowed(&caller, AccountAction::RenameComputer)?;
    let prepared = match prepare::<ComputerRenameRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let body = &prepared.body;
    rename(&state, pool, &caller.account, &body.device_id, &body.name).await?;
    let value = value_of(&ActionAck { ok: true })?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}
