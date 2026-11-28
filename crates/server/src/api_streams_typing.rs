use std::error::Error;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use http_body_util::LengthLimitError;

use anybot_bus::TypingNote;
use anybot_domain::{AcceptOutcome, decide_accept};
use anybot_messages::MessageCode;
use anybot_protocol::{StreamSelector, TypingRequest, selector_key};

use crate::api_streams_util::{denial_code, resolve_stream};
use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::req_write::{
    CallerDenied, MAX_WRITE_BODY, denied, denied_detail, idempotency_key_of, internal_text, pool_of,
};

const WRITER_KIND: &str = "message";

#[utoipa::path(
    post,
    path = "/v1/streams/typing",
    operation_id = "post_streams_typing",
    summary = "Hand the answer typed so far to the devices that are watching",
    description = "Only a computer credential that may write the Bot stream can call this. The body carries the whole answer typed so far, and the thinking so far (an empty string when there is none). The server forwards it as a typing event on the notification stream of every connection that can see the Bot stream, at most once per second per task for each connection, and keeps nothing: no database row, no record. It is a hint, not a record. The Idempotency-Key header is required; a repeat is another hint and is not stored.",
    tag = "streams",
    security(("bearer" = [])),
    params(("Idempotency-Key" = String, Header, description = "client-generated idempotency key")),
    request_body(content = TypingRequest, example = json!({"stream": {"bot": {"id": "018f-bot-uuidv7"}}, "task_id": "018f-task-uuidv7", "text": "The answer so far", "thinking": ""})),
    responses(
        (status = 204, description = "Forwarded"),
        (status = 400, description = "Bad body, or the stream is not a Bot stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "Not a computer credential, or not allowed to write this Bot stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "device-no-permission", "params": {}, "status": 403, "title": "This device cannot do that.", "type": "https://anybot.test/problems/device_no_permission"})),
        (status = 413, description = "Body larger than the write limit",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 413, "title": "The request body is too large.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    )
)]
pub async fn typing(
    State(state): State<AppState>,
    caller: Caller,
    request: Request,
) -> Result<Response, CallerDenied> {
    require_device(&caller)?;
    let _key = idempotency_key_of(request.headers())?;
    let req = read_request(request).await?;
    if !matches!(req.stream, StreamSelector::Bot { .. }) {
        return Err(denied_detail(
            StatusCode::BAD_REQUEST,
            "typing belongs to a Bot stream",
        ));
    }
    let pool = pool_of(&state)?;
    let resolved = resolve_stream(pool, &caller, &req.stream).await?;
    let disabled = resolved.account_disabled(pool).await?;
    let outcome = decide_accept(WRITER_KIND, &resolved.accept_context(&caller, disabled));
    if outcome != AcceptOutcome::Accept {
        let code = denial_code(outcome).map_err(internal_text)?;
        return Err(denied(StatusCode::FORBIDDEN, code));
    }
    state.typing.publish(TypingNote {
        stream: selector_key(&req.stream, &caller.account),
        task_id: req.task_id,
        text: req.text,
        thinking: req.thinking,
        at: state.clock.now_ms(),
    });
    Ok(StatusCode::NO_CONTENT.into_response())
}

fn require_device(caller: &Caller) -> Result<(), CallerDenied> {
    match caller.credential {
        CredentialKind::Computer => Ok(()),
        CredentialKind::Cookie | CredentialKind::Bearer | CredentialKind::Runner => Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::DeviceNoPermission,
        )),
    }
}

async fn read_request(request: Request) -> Result<TypingRequest, CallerDenied> {
    let raw = axum::body::to_bytes(request.into_body(), MAX_WRITE_BODY)
        .await
        .map_err(|e| {
            let too_large = e
                .source()
                .is_some_and(|inner| inner.downcast_ref::<LengthLimitError>().is_some());
            if too_large {
                denied_detail(StatusCode::PAYLOAD_TOO_LARGE, "typing body is too large")
            } else {
                internal_text(e.to_string())
            }
        })?;
    serde_json::from_slice(&raw).map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))
}
