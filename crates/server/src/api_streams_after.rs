use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use anybot_messages::MessageCode;
use anybot_protocol::{AfterPage, AfterRequest, MAX_AFTER_PAGE, StoredRecord, selector_from_key};
use anybot_store_server::{Pool, streams};
use serde::Deserialize;

use crate::api_streams_util::resolve_stream;
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, denied, internal_sqlx, internal_text, pool_of, value_of};

#[derive(Debug, Deserialize)]
pub struct AfterQuery {
    pub stream: String,
    pub after: i64,
    #[serde(default)]
    pub device_at: Option<String>,
}

impl AfterQuery {
    fn to_request(&self) -> Result<AfterRequest, CallerDenied> {
        let selector = selector_from_key(&self.stream)
            .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e))?;
        Ok(AfterRequest {
            stream: selector,
            after_seq: self.after,
            device_id_at_after: self.device_at.clone(),
        })
    }
}

#[utoipa::path(
    get,
    path = "/v1/streams/after",
    operation_id = "get_streams_after",
    summary = "Records after sequence N on one stream",
    description = "Carries N and the device id of record N; replies with the next page in order. A stream the caller cannot read is refused.",
    tag = "streams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Page of records",
            content_type = "application/json",
            body = AfterPage,
            example = json!({"stream": "account", "records": [{"seq": 3, "device_id": "018f-uuidv7", "kind": "message", "content": {"text": "hi"}, "created_at_ms": 1_700_000_000_100_i64, "offline_sent": false}], "has_more": false, "max_seq": 3})),
        (status = 400, description = "Bad stream key",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "No permission for this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 409, description = "Client is ahead of the server",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "sync-after-ahead", "params": {"max_seq": 45}, "status": 409, "title": "This device is ahead of the server (max_seq 45): please reset the stream.", "type": "https://anybot.test/problems/sync_after_ahead"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("stream" = String, Query, description = "stream key: account/{owner} or bot/{id}"),
        ("after" = i64, Query, description = "last sequence the client has"),
        ("device_at" = Option<String>, Query, description = "device id of record N"),
    )
)]
pub async fn after(
    State(state): State<AppState>,
    caller: Caller,
    Query(query): Query<AfterQuery>,
) -> Result<Response, CallerDenied> {
    let req = query.to_request()?;
    let pool = pool_of(&state)?;
    let resolved = resolve_stream(pool, &caller, &req.stream).await?;
    if resolved.owner != caller.account {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::RecordNoPermission,
        ));
    }
    let max = streams::max_seq(pool, &resolved.key)
        .await
        .map_err(internal_sqlx)?;
    if ahead_of_server(pool, &resolved.key, &req, max).await? {
        return Err(ahead_problem(max));
    }
    let page = build_page(pool, &resolved.key, &req, max).await?;
    let value = value_of(&page)?;
    Ok((StatusCode::OK, axum::Json(value)).into_response())
}

async fn ahead_of_server(
    pool: &Pool,
    key: &str,
    req: &AfterRequest,
    max: i64,
) -> Result<bool, CallerDenied> {
    if req.after_seq > max {
        return Ok(true);
    }
    if req.after_seq == 0 {
        return Ok(false);
    }
    let at = streams::record_at(pool, key, req.after_seq)
        .await
        .map_err(internal_sqlx)?;
    let wanted = req
        .device_id_at_after
        .as_ref()
        .ok_or(crate::req_write::internal_detail("after>0 needs device_at"))?;
    match at {
        Some(r) => Ok(r.device_id != *wanted),
        None => Ok(true),
    }
}

async fn build_page(
    pool: &Pool,
    key: &str,
    req: &AfterRequest,
    max: i64,
) -> Result<AfterPage, CallerDenied> {
    let rows = streams::list_after(pool, key, req.after_seq, MAX_AFTER_PAGE + 1)
        .await
        .map_err(internal_sqlx)?;
    let has_more =
        rows.len() > usize::try_from(MAX_AFTER_PAGE).map_err(|e| internal_text(e.to_string()))?;
    let keep = if has_more {
        usize::try_from(MAX_AFTER_PAGE).map_err(|e| internal_text(e.to_string()))?
    } else {
        rows.len()
    };
    let records = rows
        .into_iter()
        .take(keep)
        .map(stored_record)
        .collect::<Result<Vec<StoredRecord>, CallerDenied>>()?;
    Ok(AfterPage {
        stream: req.stream.clone(),
        records,
        has_more,
        max_seq: max,
    })
}

fn stored_record(row: streams::RecordRow) -> Result<StoredRecord, CallerDenied> {
    let content = serde_json::from_str(&row.content).map_err(|e| internal_text(e.to_string()))?;
    Ok(StoredRecord {
        seq: row.seq,
        device_id: row.device_id,
        kind: row.kind,
        content,
        created_at_ms: row.created_at_ms,
        offline_sent: row.offline_sent,
        task_id: row.task_id,
        task_seq: row.task_seq,
    })
}

fn ahead_problem(max: i64) -> CallerDenied {
    let params = anybot_messages::Params::new().with_int("max_seq", max);
    CallerDenied {
        problem: crate::problems::auth_problem_params(
            StatusCode::CONFLICT.as_u16(),
            MessageCode::SyncAfterAhead,
            &params,
        ),
    }
}
