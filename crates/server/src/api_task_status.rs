use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use anybot_domain::task_replay::replay;
use anybot_messages::MessageCode;
use anybot_protocol::{
    StatusRecord, StreamSelector, TaskReport, TaskStatusReply, selector_from_key,
    task_event_from_records,
};
use anybot_store_server::{streams, streams_task};

use crate::api_streams_util::resolve_stream;
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{
    CallerDenied, denied, denied_detail, internal_detail, internal_sqlx, internal_text, pool_of,
    value_of,
};

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub bot: String,
}
#[utoipa::path(
    get,
    path = "/v1/tasks/{task}/status",
    operation_id = "get_task_status",
    summary = "Read-only status of one task",
    description = "Folds the records of the bot stream that belong to this task and answers the current status, detail and report. The bot must belong to the caller; a deleted bot stays readable. No idempotency key: this endpoint writes nothing.",
    tag = "streams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The folded task status",
            content_type = "application/json",
            body = TaskStatusReply,
            example = json!({"status": "completed", "detail": {"detail": "user_confirmed"}, "report": {"userConfirmed": true, "userConfirmedAtMs": 1_700_000_000_200_i64}, "lastSeq": 9})),
        (status = 400, description = "Bad bot stream key",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "The bot belongs to another account",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 404, description = "No task with this id",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "task-not-found", "params": {}, "status": 404, "title": "This task has no records in this stream.", "type": "https://anybot.test/problems/task_not_found"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("task" = String, Path, description = "Task id of the task to read"),
        ("bot" = String, Query, description = "Bot stream key, for example bot/<id>"),
    )
)]
pub async fn task_status(
    State(state): State<AppState>,
    caller: Caller,
    Path(task): Path<String>,
    RawQuery(raw): RawQuery,
) -> Result<Response, CallerDenied> {
    if task.is_empty() || task.len() > 64 {
        return Err(task_not_found());
    }
    let task_text = task;
    let bot_text = bot_from_query(raw.as_deref())?;
    let selector = selector_from_key(&bot_text).map_err(|e| bad_request(&e))?;
    let pool = pool_of(&state)?;
    let resolved = bot_stream(pool, &caller, &selector).await?;
    let rows = streams_task::list_task_records(pool, &resolved.key, &task_text)
        .await
        .map_err(internal_sqlx)?;
    if rows.is_empty() {
        return Err(task_not_found());
    }
    let records: Vec<StatusRecord> = rows.iter().map(status_record).collect();
    let events = task_event_from_records(&records).map_err(internal_text)?;
    let view = replay(&events).map_err(|gap| {
        internal_detail(&format!(
            "task replay gap: {} missing sequences",
            gap.missing.len()
        ))
    })?;
    let last_seq = streams::max_seq(pool, &resolved.key)
        .await
        .map_err(internal_sqlx)?;
    let reply = status_reply(&view, last_seq)?;
    let value = value_of(&reply)?;
    Ok((StatusCode::OK, axum::Json(value)).into_response())
}

async fn bot_stream(
    pool: &anybot_store_server::Pool,
    caller: &Caller,
    selector: &StreamSelector,
) -> Result<crate::api_streams_util::ResolvedStream, CallerDenied> {
    if !matches!(selector, StreamSelector::Bot { id: _ }) {
        return Err(denied_detail(
            StatusCode::BAD_REQUEST,
            "the bot parameter must name a bot stream",
        ));
    }
    let resolved = resolve_stream(pool, caller, selector).await?;
    if resolved.owner != caller.account {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::RecordNoPermission,
        ));
    }
    Ok(resolved)
}

fn status_record(row: &streams::RecordRow) -> StatusRecord {
    StatusRecord {
        task_seq: row.task_seq.unwrap_or(row.seq),
        device_id: row.device_id.clone(),
        kind: row.kind.clone(),
        content: row.content.clone(),
        created_at_ms: row.created_at_ms,
    }
}

fn status_reply(
    view: &anybot_domain::TaskView,
    last_seq: i64,
) -> Result<TaskStatusReply, CallerDenied> {
    let seq = u64::try_from(last_seq).map_err(internal_text)?;
    Ok(TaskStatusReply {
        status: view.status,
        detail: view.detail.clone(),
        report: TaskReport {
            user_confirmed: view.report.user_confirmed,
            user_confirmed_at_ms: view.report.user_confirmed_at_ms,
        },
        last_seq: seq,
    })
}

fn bad_request(text: &str) -> CallerDenied {
    denied_detail(StatusCode::BAD_REQUEST, text)
}

fn bot_from_query(raw: Option<&str>) -> Result<String, CallerDenied> {
    let query = raw.ok_or_else(|| bad_request("the bot query parameter is required"))?;
    for pair in query.split('&') {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| bad_request("the bot query parameter is required"))?;
        if name == "bot" && !value.is_empty() {
            return Ok(value.to_owned());
        }
    }
    Err(bad_request("the bot query parameter is required"))
}

fn task_not_found() -> CallerDenied {
    denied(StatusCode::NOT_FOUND, MessageCode::TaskNotFound)
}
