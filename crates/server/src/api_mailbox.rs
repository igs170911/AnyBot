use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use anybot_domain::computer_actions::AccountAction;
use anybot_messages::MessageCode;
use anybot_protocol::{ClaimReply, ClaimRequest, ProblemJson, RoundReply, RoundRequest};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computers_action::{Step, finish, prepare, require_allowed};
use crate::req_write::{CallerDenied, denied, pool_of, value_of};

#[utoipa::path(
    post,
    path = "/v1/mailbox/round",
    operation_id = "post_mailbox_round",
    summary = "Hand one round to a computer's mailbox",
    description = "The server picks the computer from the credential, the optional named computer and the availability rules, puts one letter into its mailbox and sends a hint on that computer's notification stream. A computer that is offline or has the delegate switch off is replaced by the Bot's computer and a line is written on the Bot stream. Computers added with a pairing code cannot call this.",
    tag = "mailbox",
    security(("bearer" = [])),
    request_body(content = RoundRequest, example = json!({"bot": "018f-bot-uuidv7", "computer": null})),
    responses(
        (status = 200, description = "The letter and where it went",
            content_type = "application/json",
            body = RoundReply,
            example = json!({"letter_id": "018f-letter-uuidv7", "device_id": "018f-device-uuidv7", "reassigned": "offline"})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 403, description = "A paired computer, the runner service, or a Bot that is not the caller's",
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
pub async fn deliver_round(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    require_allowed(&caller, AccountAction::DeliverRound)?;
    let prepared = match prepare::<RoundRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let reply = crate::mailbox_deliver::deliver(&state, pool, &caller, &prepared.body).await?;
    let value = value_of(&reply)?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}

#[utoipa::path(
    post,
    path = "/v1/mailbox/claim",
    operation_id = "post_mailbox_claim",
    summary = "Claim one letter from the calling computer's mailbox",
    description = "The server lets exactly one claim win, with one conditional update. A refused claim says why: already reassigned, taken by another computer, already claimed by the caller, or withdrawn. A letter that waited ten seconds without being claimed is reassigned to the Bot's computer before the claim is decided. Only computers and the runner service can call this.",
    tag = "mailbox",
    security(("bearer" = [])),
    request_body(content = ClaimRequest, example = json!({"letter_id": "018f-letter-uuidv7"})),
    responses(
        (status = 200, description = "The letter content",
            content_type = "application/json",
            body = ClaimReply,
            example = json!({"letter_id": "018f-letter-uuidv7", "kind": "round", "content": {"bot": "018f-bot-uuidv7"}})),
        (status = 400, description = "Missing idempotency key",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/account_idempotency_missing", "title": "The Idempotency-Key header is required.", "status": 400, "code": "account-idempotency-missing", "params": {}})),
        (status = 403, description = "Not a computer or the runner service",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/device_no_permission", "title": "This device has no permission to do this", "status": 403, "code": "device-no-permission", "params": {}})),
        (status = 404, description = "No such letter for this computer",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/mailbox_letter_unknown", "title": "There is no such letter for this computer", "status": 404, "code": "mailbox-letter-unknown", "params": {}})),
        (status = 409, description = "Reassigned, taken by another computer, already yours, withdrawn, or an idempotency key conflict",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/mailbox_claim_reassigned", "title": "This letter was reassigned to another computer: do not run it", "status": 409, "code": "mailbox-claim-reassigned", "params": {}})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/problem_internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "client-generated idempotency key")
    )
)]
pub async fn claim_letter(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    if !matches!(
        caller.credential,
        CredentialKind::Computer | CredentialKind::Runner
    ) {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::DeviceNoPermission,
        ));
    }
    let prepared = match prepare::<ClaimRequest>(&state, &caller, request).await? {
        Step::Replay(response) => return Ok(response),
        Step::Run(prepared) => prepared,
    };
    let pool = pool_of(&state)?;
    let reply =
        crate::mailbox_claim::claim(&state, pool, &caller, &prepared.body.letter_id).await?;
    let value = value_of(&reply)?;
    finish(&state, &prepared, StatusCode::OK, &value).await
}
