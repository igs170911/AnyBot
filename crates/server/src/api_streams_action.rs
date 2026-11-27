use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use anybot_domain::{AcceptOutcome, decide_accept};
use anybot_protocol::{
    RecordAction, RecordActionReply, RecordActionRequest, StreamSelector, selector_key,
};
use anybot_store_server::{Pool, streams, streams_action};

use crate::api_streams_action_plan::{ActionPlan, action_device_id, plan_action, purge_range};
use crate::api_streams_append::replay_guard;
use crate::api_streams_util::{denial_code, resolve_stream};
use crate::app::AppState;
use crate::caller::Caller;
use crate::idem::request_scope;
use crate::req_write::{
    CallerDenied, Outcome, denied, denied_detail, idempotency_key_of, internal_sqlx, internal_text,
    pool_of, record_outcome, value_of,
};

#[derive(Serialize)]
struct ActionFingerprint {
    stream: String,
    action: RecordAction,
    target_seq: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/v1/streams/actions",
    operation_id = "post_streams_actions",
    summary = "Delete, clear, restore or permanently delete chat records",
    description = "The server records the action itself. Delete and clear hide chat messages and model answers; restore brings them back within 30 days; purge-permanent empties the content of the chat records behind a delete or clear, and keeps their rows. Only the owner of the stream can act on it.\nThe Idempotency-Key header is required; the fingerprint covers the stream key, the action and the target, never any content.",
    tag = "streams",
    security(("bearer" = [])),
    request_body(content = RecordActionRequest, example = json!({"stream": "account", "action": "clear", "target_seq": null})),
    responses(
        (status = 200, description = "Sequence of the action record",
            content_type = "application/json",
            body = RecordActionReply,
            example = json!({"stream": "account", "seq": 9})),
        (status = 400, description = "Bad body, missing target, or the action cannot apply to that record",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-action-target-invalid", "params": {}, "status": 400, "title": "This action cannot be applied to that record.", "type": "https://anybot.test/problems/record_action_target_invalid"})),
        (status = 403, description = "Not allowed to act on this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 404, description = "Target record is not in this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-action-target-missing", "params": {}, "status": 404, "title": "The record to act on is not in this stream.", "type": "https://anybot.test/problems/record_action_target_missing"})),
        (status = 409, description = "Restore after the 30-day window, or same key with a different fingerprint",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-restore-expired", "params": {}, "status": 409, "title": "The 30-day window to restore this has passed.", "type": "https://anybot.test/problems/record_restore_expired"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "UUID of this action"),
    )
)]
pub async fn action(
    State(state): State<AppState>,
    caller: Caller,
    request: axum::extract::Request,
) -> Result<Response, CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY)
        .await
        .map_err(|e| internal_text(e.to_string()))?
        .to_vec();
    let key = idempotency_key_of(&parts.headers)?;
    let req: RecordActionRequest = serde_json::from_slice(&raw)
        .map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let scope = scope_of(&parts, &caller, &req.stream);
    let print = fingerprint(&req, &caller.account)?;
    if let Some(answer) = replay_guard(pool, &key, &print, &scope).await? {
        return Ok(answer);
    }
    let reply = run_action(&state, pool, &caller, &req, &key).await?;
    let value = value_of(&reply)?;
    record_outcome(
        pool,
        &state,
        &Outcome {
            key: &key,
            raw: &print,
            scope: &scope,
            status: StatusCode::OK,
            value: &value,
            marker: false,
        },
    )
    .await?;
    Ok((StatusCode::OK, axum::Json(value)).into_response())
}

pub(crate) fn scope_of(
    parts: &axum::http::request::Parts,
    caller: &Caller,
    stream: &StreamSelector,
) -> String {
    let identity = format!(
        "{}|{}",
        caller.account,
        selector_key(stream, &caller.account)
    );
    request_scope(&Method::POST, parts.uri.path(), &identity)
}

fn fingerprint(req: &RecordActionRequest, owner: &str) -> Result<Vec<u8>, CallerDenied> {
    let print = ActionFingerprint {
        stream: selector_key(&req.stream, owner),
        action: req.action,
        target_seq: req.target_seq,
    };
    serde_json::to_vec(&print).map_err(|e| internal_text(e.to_string()))
}

async fn run_action(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    req: &RecordActionRequest,
    key: &str,
) -> Result<RecordActionReply, CallerDenied> {
    let resolved = resolve_stream(pool, caller, &req.stream).await?;
    let disabled = resolved.account_disabled(pool).await?;
    let outcome = decide_accept("message", &resolved.accept_context(caller, disabled));
    if outcome != AcceptOutcome::Accept {
        let code = denial_code(outcome).map_err(internal_text)?;
        return Err(denied(StatusCode::FORBIDDEN, code));
    }
    let now =
        i64::try_from(state.clock.now_ms().millis()).map_err(|e| internal_text(e.to_string()))?;
    let mut tx = streams_action::begin(pool).await.map_err(internal_sqlx)?;
    let max = streams_action::lock_max_seq(&mut tx, &resolved.key)
        .await
        .map_err(internal_sqlx)?;
    let plan = plan_action(&mut tx, &resolved.key, req, max, now).await?;
    if let Some((low, high)) = plan.purge {
        purge_range(&mut tx, &resolved.key, low, high).await?;
    }
    let seq = insert_action(&mut tx, &resolved.key, &plan, &action_device_id(key), now).await?;
    tx.commit().await.map_err(internal_sqlx)?;
    publish_note(state, &resolved.key, seq);
    Ok(RecordActionReply {
        stream: req.stream.clone(),
        seq,
    })
}

async fn insert_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stream_key: &str,
    plan: &ActionPlan,
    device_id: &str,
    now: i64,
) -> Result<i64, CallerDenied> {
    let spec = streams::InsertSpec {
        device_id,
        kind: plan.kind,
        content: &plan.content,
        created_at_ms: now,
        offline_sent: false,
        task_id: None,
    };
    streams::insert_in_tx(tx, stream_key, &spec)
        .await
        .map_err(internal_sqlx)
}

pub(crate) fn publish_note(state: &AppState, key: &str, seq: i64) {
    let note =
        anybot_bus::Notification::with_seq(&anybot_bus::stream_note(key), &*state.clock, seq);
    state.bus.publish(note);
}
