use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use anybot_messages::MessageCode;
use anybot_protocol::{StoredRecord, selector_from_key};
use anybot_store_server::streams;

use crate::api_streams_after::stored_record;
use crate::api_streams_util::resolve_stream;
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{CallerDenied, denied, denied_detail, internal_sqlx, pool_of, value_of};

#[derive(Debug, Deserialize)]
pub struct RecordQuery {
    pub stream: String,
    pub seq: i64,
}

#[utoipa::path(
    get,
    path = "/v1/streams/record",
    operation_id = "get_streams_record",
    summary = "One record by sequence number",
    description = "Reads one record of a stream the caller owns. A record that was permanently deleted is refused with record-source-purged, so an evidence reference can show that its source is gone.",
    tag = "streams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "The record",
            content_type = "application/json",
            body = StoredRecord,
            example = json!({"seq": 3, "device_id": "018f-uuidv7", "kind": "message", "content": {"text": "hi"}, "created_at_ms": 1_700_000_000_100_i64, "offline_sent": false})),
        (status = 400, description = "Bad stream key",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-bad-request", "params": {}, "status": 400, "title": "The request was malformed.", "type": "https://anybot.test/problems/bad-request"})),
        (status = 403, description = "No permission for this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 404, description = "No record with this sequence number",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-action-target-missing", "params": {}, "status": 404, "title": "The record to act on is not in this stream.", "type": "https://anybot.test/problems/record_action_target_missing"})),
        (status = 410, description = "The record was permanently deleted",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-source-purged", "params": {}, "status": 410, "title": "The source was permanently deleted.", "type": "https://anybot.test/problems/record_source_purged"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("stream" = String, Query, description = "Stream key, for example account/alice or bot/<id>"),
        ("seq" = i64, Query, description = "Sequence number of the record"),
    )
)]
pub async fn record(
    State(state): State<AppState>,
    caller: Caller,
    Query(query): Query<RecordQuery>,
) -> Result<Response, CallerDenied> {
    let selector =
        selector_from_key(&query.stream).map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e))?;
    let pool = pool_of(&state)?;
    let resolved = resolve_stream(pool, &caller, &selector).await?;
    if resolved.owner != caller.account {
        return Err(denied(
            StatusCode::FORBIDDEN,
            MessageCode::RecordNoPermission,
        ));
    }
    let row = streams::record_at(pool, &resolved.key, query.seq)
        .await
        .map_err(internal_sqlx)?
        .ok_or_else(|| {
            denied(
                StatusCode::NOT_FOUND,
                MessageCode::RecordActionTargetMissing,
            )
        })?;
    if row.purged {
        return Err(denied(StatusCode::GONE, MessageCode::RecordSourcePurged));
    }
    let value = value_of(&stored_record(row)?)?;
    Ok((StatusCode::OK, axum::Json(value)).into_response())
}
