use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use futures_util::{
    StreamExt,
    future::{self, Ready},
};
use serde::Deserialize;
use tokio::sync::broadcast;

use anybot_bus::Notification;
use anybot_messages::MessageCode;
use anybot_protocol::{
    HEARTBEAT_INTERVAL_SECS, NewRecordsEvent, ProgressEvent, SseStreamProgress, StreamProgress,
    selector_from_key, sse_event_line, sse_heartbeat_line,
};
use anybot_store_server::{Pool, streams};

use crate::api_streams_util::resolve_stream;
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, denied, internal_sqlx, internal_text, pool_of};

#[derive(Debug, Deserialize)]
pub struct SseQuery {
    pub stream: String,
    #[serde(default)]
    pub init_seq: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/v1/streams/notifications",
    operation_id = "get_streams_notifications",
    summary = "Notification stream of one record stream (SSE)",
    description = "Stays open: the first event is progress carrying the current max seq of this stream; then new-records and progress events; a heartbeat line every 15 seconds; and one final stream-progress event with the current max seq when the server closes the stream. init_seq replays the records after it. X-Accel-Buffering is set to no.",
    tag = "streams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "event stream",
            content_type = "text/event-stream",
            example = "event: progress\ndata: {\"streams\":[{\"stream\":\"account\",\"seq\":45}]}\n\n:hb\n\nevent: new-records\ndata: {\"stream\":\"account\",\"seq\":46,\"device_id\":\"018f-uuidv7\"}\n\n"),
        (status = 400, description = "Bad stream key",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "No permission for this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("stream" = String, Query, description = "stream key: account/{owner} or bot/{id}"),
        ("init_seq" = Option<i64>, Query, description = "replay records after this sequence"),
    )
)]
pub async fn notifications(
    State(state): State<AppState>,
    caller: Caller,
    Query(query): Query<SseQuery>,
) -> Result<Response, CallerDenied> {
    let selector = selector_from_key(&query.stream)
        .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e))?;
    let pool = pool_of(&state)?;
    let resolved = resolve_stream(pool, &caller, &selector).await?;
    if resolved.owner != caller.account {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::RecordNoPermission,
        ));
    }
    let head = head_lines(pool, &resolved.key, query.init_seq).await?;
    let tail = Tail {
        rx: state.bus.subscribe(),
        key: resolved.key.clone(),
        pool: pool.clone(),
        shutdown: std::sync::Arc::clone(&state.shutdown),
        clock: std::sync::Arc::clone(&state.clock),
        last_seq: query.init_seq.unwrap_or(0),
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

async fn head_lines(
    pool: &Pool,
    key: &str,
    init_seq: Option<i64>,
) -> Result<Vec<String>, CallerDenied> {
    let max = streams::max_seq(pool, key).await.map_err(internal_sqlx)?;
    let mut out = vec![progress_line(key, max)];
    let Some(init) = init_seq else {
        return Ok(out);
    };
    let rows = streams::list_after(pool, key, init, anybot_protocol::MAX_AFTER_PAGE)
        .await
        .map_err(internal_sqlx)?;
    for row in rows {
        let event = NewRecordsEvent {
            stream: key.to_owned(),
            seq: row.seq,
            device_id: row.device_id,
        };
        let json = serde_json::to_string(&event).map_err(internal_text)?;
        out.push(sse_event_line("new-records", &json));
    }
    Ok(out)
}

fn progress_line(key: &str, max: i64) -> String {
    let event = ProgressEvent {
        streams: vec![StreamProgress {
            stream: key.to_owned(),
            seq: max,
            device_id: String::new(),
        }],
    };
    let json = serde_json::to_string(&event).unwrap_or_default();
    sse_event_line("progress", &json)
}

struct Tail {
    rx: broadcast::Receiver<Notification>,
    key: String,
    pool: Pool,
    shutdown: std::sync::Arc<tokio::sync::Notify>,
    clock: std::sync::Arc<dyn anybot_foundation::Clock>,
    last_seq: i64,
}

async fn drive(head: Vec<String>, tail: Tail, tx: tokio::sync::mpsc::Sender<String>) {
    for line in head {
        if tx.send(line).await.is_err() {
            return;
        }
    }
    let ending = tail.shutdown.notified();
    tokio::pin!(ending);
    let mut rx = tail.rx;
    let key = tail.key;
    let pool = tail.pool;
    let clock = tail.clock;
    let mut last_seq = tail.last_seq;
    loop {
        let step = tokio::select! {
            () = &mut ending => Step::End,
            () = clock.sleep(std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS)) => {
                Step::Beat
            }
            note = rx.recv() => on_note(&key, &pool, &mut last_seq, note).await,
        };
        match step {
            Step::End => break,
            Step::Beat => {
                if tx.send(sse_heartbeat_line().to_owned()).await.is_err() {
                    return;
                }
            }
            Step::Push(line) => {
                if tx.send(line).await.is_err() {
                    return;
                }
            }
            Step::Skip => {}
        }
    }
    let closing = SseStreamProgress {
        stream: key,
        max_seq: last_seq,
    };
    send_closing(&closing, &tx).await;
}

enum Step {
    End,
    Beat,
    Push(String),
    Skip,
}

async fn on_note(
    key: &str,
    pool: &Pool,
    last_seq: &mut i64,
    note: Result<Notification, broadcast::error::RecvError>,
) -> Step {
    if note.map(|n| n.kind).unwrap_or_default() != key {
        return Step::Skip;
    }
    let Ok(top) = streams::max_seq(pool, key).await else {
        return Step::Skip;
    };
    *last_seq = top;
    let device = device_at(pool, key, top).await;
    let event = NewRecordsEvent {
        stream: key.to_owned(),
        seq: top,
        device_id: device,
    };
    let Ok(line) = json_line("new-records", &event) else {
        return Step::Skip;
    };
    Step::Push(line)
}

async fn send_closing(event: &SseStreamProgress, tx: &tokio::sync::mpsc::Sender<String>) {
    match json_line("stream-progress", event) {
        Ok(line) => {
            let _sent = tx.send(line).await;
        }
        Err(e) => tracing::warn!("sse closing encode failed: {e}"),
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

fn line_result(line: String) -> Ready<Result<String, std::io::Error>> {
    future::ready(Ok(line))
}

fn json_line<T: serde::Serialize>(name: &str, event: &T) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(event)?;
    Ok(sse_event_line(name, &json))
}

fn open_stream(head: Vec<String>, tail: Tail) -> tokio::sync::mpsc::Receiver<String> {
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    tokio::spawn(drive(head, tail, tx));
    rx
}
