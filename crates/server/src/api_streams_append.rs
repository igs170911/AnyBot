use axum::extract::State;
use axum::http::Method;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use anybot_domain::{AcceptOutcome, decide_accept};
use anybot_messages::MessageCode;
use anybot_protocol::StreamSelector;
use anybot_protocol::{AppendReply, AppendResult, RecordItem, SETTING_KIND, selector_key};
use anybot_store_server::{Pool, streams, streams_action};

use crate::api_streams_dedup::duplicate_result;
use crate::api_streams_util::{ResolvedStream, denial_code, resolve_stream, too_many_items};
use crate::app::AppState;
use crate::caller::Caller;
use crate::idem::{find_recorded, replay_value, request_scope, safe_digest};
use crate::req_write::{
    CallerDenied, Outcome, denied, denied_detail, idempotency_key_of, internal_sqlx, internal_text,
    pool_of, record_outcome, value_of,
};

#[derive(Serialize)]
struct AppendFingerprint {
    stream: String,
    device_ids: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/v1/streams/append",
    operation_id = "post_streams_append",
    summary = "Append records to one stream",
    description = "Carries up to 1000 records with the device id each one was born with. The server assigns one sequence per record, rejects forbidden kinds, and answers the same for a replay of the same key.\nThe Idempotency-Key header is required: the fingerprint covers the stream key and the sorted device ids of the batch.",
    tag = "streams",
    security(("bearer" = [])),
    request_body(content = anybot_protocol::AppendRequest, example = json!({"stream": "account", "items": [{"device_id": "018f-uuidv7", "kind": "message", "content": {"text": "hi"}, "created_at_ms": 1_700_000_000_100_i64, "offline_sent": false}]})),
    responses(
        (status = 200, description = "Per-item results in request order",
            content_type = "application/json",
            body = AppendReply,
            example = json!({"stream": "account", "results": [{"result": "accepted", "seq": 1}, {"result": "duplicate", "seq": 1}, {"result": "rejected", "code": "record-id-conflict"}]})),
        (status = 400, description = "Bad body or missing idempotency key",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "Not allowed to write this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 409, description = "Same key with a different fingerprint",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "account-idempotency-conflict", "params": {}, "status": 409, "title": "This idempotency key was already used for a different request.", "type": "https://anybot.test/problems/account_idempotency_conflict"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "UUID of this batch"),
    )
)]
pub async fn append(
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
    let req: anybot_protocol::AppendRequest = read_raw(&raw)?;
    if too_many_items(&req.items) {
        return Err(denied_detail(
            StatusCode::BAD_REQUEST,
            "append batch is too large",
        ));
    }
    let pool = pool_of(&state)?;
    let scope = scope_of(&parts, &caller, &req);
    let print = fingerprint_body(&req, &caller.account)?;
    if let Some(answer) = replay_guard(pool, &key, &print, &scope).await? {
        return Ok(answer);
    }
    let work = AppendWork {
        state: &state,
        pool,
        caller: &caller,
        req: &req,
    };
    let reply = run_append(&work).await?;
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

struct AppendWork<'a> {
    state: &'a AppState,
    pool: &'a Pool,
    caller: &'a Caller,
    req: &'a anybot_protocol::AppendRequest,
}

fn read_raw(raw: &[u8]) -> Result<anybot_protocol::AppendRequest, CallerDenied> {
    serde_json::from_slice(raw).map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))
}

fn scope_of(
    parts: &axum::http::request::Parts,
    caller: &Caller,
    req: &anybot_protocol::AppendRequest,
) -> String {
    let identity = format!(
        "{}|{}",
        caller.account,
        selector_key(&req.stream, &caller.account)
    );
    request_scope(&Method::POST, parts.uri.path(), &identity)
}

fn fingerprint_body(
    req: &anybot_protocol::AppendRequest,
    owner: &str,
) -> Result<Vec<u8>, CallerDenied> {
    let mut ids: Vec<String> = req.items.iter().map(|i| i.device_id.clone()).collect();
    ids.sort();
    let print = AppendFingerprint {
        stream: selector_key(&req.stream, owner),
        device_ids: ids,
    };
    serde_json::to_vec(&print).map_err(|e| internal_text(e.to_string()))
}

pub(crate) async fn replay_guard(
    pool: &Pool,
    key: &str,
    raw: &[u8],
    scope: &str,
) -> Result<Option<Response>, CallerDenied> {
    let stored = find_recorded(pool, key, scope)
        .await
        .map_err(internal_sqlx)?;
    match stored {
        None => Ok(None),
        Some(record) if record.request_hash == safe_digest(raw)? => {
            let value = replay_value(&record.body)?;
            let status = crate::api_register::status_from_i32(record.status_code)?;
            Ok(Some((status, axum::Json(value)).into_response()))
        }
        Some(_) => Err(denied(
            StatusCode::CONFLICT,
            MessageCode::AccountIdempotencyConflict,
        )),
    }
}

async fn run_append(work: &AppendWork<'_>) -> Result<AppendReply, CallerDenied> {
    ensure_account_stream(work).await?;
    let resolved = resolve_stream(work.pool, work.caller, &work.req.stream).await?;
    let disabled = resolved.account_disabled(work.pool).await?;
    let mut results = Vec::with_capacity(work.req.items.len());
    for item in &work.req.items {
        results.push(append_one(work, &resolved, disabled, item).await?);
    }
    publish_notes(work, &resolved, &results);
    Ok(AppendReply {
        stream: work.req.stream.clone(),
        results,
    })
}

async fn ensure_account_stream(work: &AppendWork<'_>) -> Result<(), CallerDenied> {
    if !matches!(work.req.stream, StreamSelector::Account) {
        return Ok(());
    }
    let now = i64::try_from(work.state.clock.now_ms().millis())
        .map_err(|e| internal_text(e.to_string()))?;
    let key = selector_key(&work.req.stream, &work.caller.account);
    streams::ensure_stream(work.pool, &key, "account", &work.caller.account, None, now)
        .await
        .map_err(internal_sqlx)
}

async fn append_one(
    work: &AppendWork<'_>,
    resolved: &ResolvedStream,
    disabled: bool,
    item: &RecordItem,
) -> Result<AppendResult, CallerDenied> {
    let existing = streams::find_record_by_device(work.pool, &resolved.key, &item.device_id)
        .await
        .map_err(internal_sqlx)?;
    if let Some(existing) = existing {
        return duplicate_result(existing, item);
    }
    let outcome = decide_accept(&item.kind, &resolved.accept_context(work.caller, disabled));
    if outcome != AcceptOutcome::Accept {
        let code = denial_code(outcome).map_err(internal_text)?;
        return Ok(AppendResult::rejected(code.as_str().to_owned()));
    }
    if item.kind == SETTING_KIND {
        return setting_item(work, resolved, item).await;
    }
    insert_item(work, resolved, item).await
}

async fn setting_item(
    work: &AppendWork<'_>,
    resolved: &ResolvedStream,
    item: &RecordItem,
) -> Result<AppendResult, CallerDenied> {
    if resolved.bot.is_some() {
        let code = MessageCode::AppendKindForbidden;
        return Ok(AppendResult::rejected(code.as_str().to_owned()));
    }
    let now = i64::try_from(work.state.clock.now_ms().millis())
        .map_err(|e| internal_text(e.to_string()))?;
    crate::api_settings_write::accept_setting(work.pool, &resolved.key, item, now).await
}

async fn insert_item(
    work: &AppendWork<'_>,
    resolved: &ResolvedStream,
    item: &RecordItem,
) -> Result<AppendResult, CallerDenied> {
    let content = serde_json::to_string(&item.content).map_err(internal_text)?;
    let spec = streams::InsertSpec {
        device_id: &item.device_id,
        kind: &item.kind,
        content: &content,
        created_at_ms: item.created_at_ms,
        offline_sent: item.offline_sent,
        task_id: item.task_id.as_deref(),
    };
    let mut tx = streams_action::begin(work.pool)
        .await
        .map_err(internal_sqlx)?;
    let seq = streams::insert_in_tx(&mut tx, &resolved.key, &spec)
        .await
        .map_err(internal_sqlx)?;
    crate::api_streams_bot::project_bot_record(&mut tx, resolved, item).await?;
    tx.commit().await.map_err(internal_sqlx)?;
    Ok(AppendResult::accepted(seq))
}

fn publish_notes(work: &AppendWork<'_>, resolved: &ResolvedStream, results: &[AppendResult]) {
    let seqs: Vec<i64> = results
        .iter()
        .filter_map(AppendResult::accepted_seq)
        .collect();
    let Some(top) = seqs.iter().max() else {
        return;
    };
    let note = anybot_bus::Notification::with_seq(
        &anybot_bus::stream_note(&resolved.key),
        &*work.state.clock,
        *top,
    );
    work.state.bus.publish(note);
}
