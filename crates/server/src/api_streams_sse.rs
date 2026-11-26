use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use futures_util::StreamExt;
use serde::Deserialize;

use anybot_protocol::{NewRecordsEvent, ProgressEvent, StreamProgress, sse_event_line};
use anybot_store_server::{Pool, streams, streams_bots};

use crate::api_streams_sse_drive::{Tail, json_line, line_result, open_stream};
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, internal_sqlx, internal_text, pool_of};

#[derive(Debug, Deserialize)]
pub struct SseQuery {
    #[serde(default)]
    pub init_seq: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/v1/streams/notifications",
    operation_id = "get_streams_notifications",
    summary = "Notification stream of every record stream the caller can see (SSE)",
    description = "Stays open and carries notifications for every stream this caller can see, never record content. The first event is progress: for each stream the current seq and which record that seq is. Then new-records events for whichever stream gained records. A heartbeat line every 15 seconds. One final stream-progress event with the current seq of every stream when the server closes the stream, including when credentials stop working. There is no N on the request and no Last-Event-ID: data is fetched with GET /v1/streams/after. X-Accel-Buffering is set to no.",
    tag = "streams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "event stream",
            content_type = "text/event-stream",
            example = "event: progress\ndata: {\"streams\":[{\"stream\":\"account/alice\",\"seq\":45,\"device_id\":\"018f-uuidv7\"}]}\n\n:hb\n\nevent: new-records\ndata: {\"stream\":\"account/alice\",\"seq\":46,\"device_id\":\"018f-uuidv7\"}\n\n"),
        (status = 400, description = "Bad query",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("init_seq" = Option<i64>, Query, description = "replay account records after this sequence"),
    )
)]
pub async fn notifications(
    State(state): State<AppState>,
    caller: Caller,
    Query(query): Query<SseQuery>,
) -> Result<Response, CallerDenied> {
    let pool = pool_of(&state)?;
    let keys = visible_keys(pool, &caller).await?;
    let init_seq = query.init_seq.unwrap_or(0);
    let head = head_lines(pool, &keys, init_seq).await?;
    let tail = Tail {
        rx: state.bus.subscribe(),
        pool: pool.clone(),
        shutdown: std::sync::Arc::clone(&state.shutdown),
        clock: std::sync::Arc::clone(&state.clock),
        account: caller.account.clone(),
        keys,
        init_seq,
    };
    let lines = open_stream(head, tail);
    let stream = tokio_stream::wrappers::ReceiverStream::new(lines).then(line_result);
    let body = axum::body::Body::from_stream(Box::pin(stream));
    Ok((
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/event-stream"),
            (
                axum::http::header::HeaderName::from_static("x-accel-buffering"),
                "no",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response())
}

async fn visible_keys(pool: &Pool, caller: &Caller) -> Result<Vec<String>, CallerDenied> {
    let account_key = format!("account/{}", caller.account);
    let mut keys = vec![account_key];
    let bots = streams_bots::list_bots(pool, &caller.account)
        .await
        .map_err(internal_sqlx)?;
    for bot in bots {
        if !bot.deleted {
            keys.push(format!("bot/{}", bot.id));
        }
    }
    Ok(keys)
}

async fn head_lines(
    pool: &Pool,
    keys: &[String],
    init_seq: i64,
) -> Result<Vec<String>, CallerDenied> {
    let mut rows = Vec::new();
    for key in keys {
        let max = max_or_new(pool, key).await?;
        let device_id = device_at(pool, key, max).await;
        rows.push(StreamProgress {
            stream: key.clone(),
            seq: max,
            device_id,
        });
    }
    let mut out = vec![progress_line(&rows)];
    if init_seq == 0 {
        return Ok(out);
    }
    let account_key = keys.first().cloned().unwrap_or_default();
    let replay = streams::list_after(
        pool,
        &account_key,
        init_seq,
        anybot_protocol::MAX_AFTER_PAGE,
    )
    .await
    .map_err(internal_sqlx)?;
    for row in replay {
        let event = NewRecordsEvent {
            stream: account_key.clone(),
            seq: row.seq,
            device_id: row.device_id,
        };
        let line = json_line("new-records", &event).map_err(internal_text)?;
        out.push(line);
    }
    Ok(out)
}

fn progress_line(rows: &[StreamProgress]) -> String {
    let event = ProgressEvent {
        streams: rows.to_vec(),
    };
    let json = serde_json::to_string(&event).unwrap_or_default();
    sse_event_line("progress", &json)
}

async fn max_or_new(pool: &Pool, key: &str) -> Result<i64, CallerDenied> {
    match streams::max_seq(pool, key).await {
        Ok(max) => Ok(max),
        Err(e) if e.to_string().contains("stream row missing") => Ok(0),
        Err(e) => Err(internal_sqlx(e)),
    }
}

async fn device_at(pool: &Pool, key: &str, seq: i64) -> String {
    match streams::record_at(pool, key, seq).await {
        Ok(Some(row)) => row.device_id,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!("sse record_at failed: {e}");
            String::new()
        }
    }
}
