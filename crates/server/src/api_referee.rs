use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use anybot_domain::referee::referee_record_kind;
use anybot_domain::{AcceptOutcome, decide_accept};
use anybot_messages::{MessageCode, Params};
use anybot_protocol::{RefereeAction, RefereeReply, RefereeRequest, selector_key};
use anybot_store_server::streams_card::{CardClaim, claim_card_in};
use anybot_store_server::{Pool, streams, streams_action};

use crate::api_streams_action::{publish_note, scope_of};
use crate::api_streams_append::replay_guard;
use crate::api_streams_util::{denial_code, resolve_stream};
use crate::app::AppState;
use crate::caller::Caller;
use crate::req_write::{
    CallerDenied, Outcome, denied, denied_with, idempotency_key_of, internal_sqlx, internal_text,
    pool_of, record_outcome, value_of,
};

#[derive(Serialize)]
struct RefereeFingerprint<'a> {
    stream: String,
    task_id: &'a str,
    action: RefereeAction,
    card_id: Option<&'a str>,
    answer: Option<&'a str>,
    criteria: Option<&'a serde_json::Value>,
}

#[utoipa::path(
    post,
    path = "/v1/streams/referee",
    operation_id = "post_streams_referee",
    summary = "Cancel, stop, wake, answer a card or change the done criteria of a task",
    description = "A device asks the server to decide. The server records the decision on the bot stream with the task id. A card answer is accepted once per card: the second answer is refused and names the device that answered first.\nThe Idempotency-Key header is required.",
    tag = "streams",
    security(("bearer" = [])),
    request_body(content = RefereeRequest, example = json!({"stream": {"bot": {"id": "018f-bot-uuidv7"}}, "task_id": "018f-task-uuidv7", "action": "card_answer", "card_id": "018f-card-uuidv7", "answer": "allow"})),
    responses(
        (status = 200, description = "Sequence of the decision record",
            content_type = "application/json",
            body = RefereeReply,
            example = json!({"stream": {"bot": {"id": "018f-bot-uuidv7"}}, "seq": 12})),
        (status = 400, description = "Bad body or a field this action needs is missing",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "referee-malformed", "params": {}, "status": 400, "title": "This referee request is missing a field it needs.", "type": "https://anybot.test/problems/referee_malformed"})),
        (status = 403, description = "Not allowed to act on this stream",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "record-no-permission", "params": {}, "status": 403, "title": "You cannot write this stream.", "type": "https://anybot.test/problems/record_no_permission"})),
        (status = 409, description = "The card was already answered, or the page is out of date",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "card-already-answered", "params": {"device": "018f-uuidv7"}, "status": 409, "title": "This card was already answered from 018f-uuidv7.", "type": "https://anybot.test/problems/card_already_answered"})),
        (status = 500, description = "Internal error",
            content_type = "application/problem+json",
            body = anybot_protocol::ProblemJson,
            example = json!({"code": "problem-internal", "params": {}, "status": 500, "title": "The server hit an internal error.", "type": "https://anybot.test/problems/internal"})),
    ),
    params(
        ("Idempotency-Key" = String, Header, description = "UUID of this decision"),
    )
)]
pub async fn referee(
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
    let req: RefereeRequest = serde_json::from_slice(&raw)
        .map_err(|e| crate::req_write::denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let pool = pool_of(&state)?;
    let scope = scope_of(&parts, &caller, &req.stream);
    let print = fingerprint(&req, &caller.account)?;
    if let Some(answer) = replay_guard(pool, &key, &print, &scope).await? {
        return Ok(answer);
    }
    let reply = run_referee(&state, pool, &caller, &req, &key).await?;
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

fn fingerprint(req: &RefereeRequest, owner: &str) -> Result<Vec<u8>, CallerDenied> {
    let print = RefereeFingerprint {
        stream: selector_key(&req.stream, owner),
        task_id: &req.task_id,
        action: req.action,
        card_id: req.card_id.as_deref(),
        answer: req.answer.as_deref(),
        criteria: req.criteria.as_ref(),
    };
    serde_json::to_vec(&print).map_err(|e| internal_text(e.to_string()))
}

async fn run_referee(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    req: &RefereeRequest,
    key: &str,
) -> Result<RefereeReply, CallerDenied> {
    let card = card_of(req)?;
    let resolved = resolve_stream(pool, caller, &req.stream).await?;
    if resolved.bot.is_none() {
        return Err(malformed());
    }
    let disabled = resolved.account_disabled(pool).await?;
    let outcome = decide_accept("message", &resolved.accept_context(caller, disabled));
    if outcome != AcceptOutcome::Accept {
        let code = denial_code(outcome).map_err(internal_text)?;
        return Err(denied(StatusCode::FORBIDDEN, code));
    }
    let now =
        i64::try_from(state.clock.now_ms().millis()).map_err(|e| internal_text(e.to_string()))?;
    let content = serde_json::to_string(req).map_err(|e| internal_text(e.to_string()))?;
    let record_device = format!("referee:{key}");
    let mut tx = streams_action::begin(pool).await.map_err(internal_sqlx)?;
    streams_action::lock_max_seq(&mut tx, &resolved.key)
        .await
        .map_err(internal_sqlx)?;
    let spec = streams::InsertSpec {
        device_id: &record_device,
        kind: referee_record_kind(req.action),
        content: &content,
        created_at_ms: now,
        offline_sent: false,
        task_id: Some(&req.task_id),
    };
    let seq = streams::insert_in_tx(&mut tx, &resolved.key, &spec)
        .await
        .map_err(internal_sqlx)?;
    if let Some((card_id, answer)) = card {
        let claim = CardClaim {
            stream_id: &resolved.key,
            card_id,
            device_id: &caller.device_id,
            answer,
            seq,
        };
        if let Some(first) = claim_card_in(&mut tx, &claim)
            .await
            .map_err(internal_sqlx)?
        {
            return Err(already_answered(&first));
        }
    }
    tx.commit().await.map_err(internal_sqlx)?;
    publish_note(state, &resolved.key, seq);
    Ok(RefereeReply {
        stream: req.stream.clone(),
        seq,
    })
}

fn card_of(req: &RefereeRequest) -> Result<Option<(&str, &str)>, CallerDenied> {
    if req.task_id.is_empty() {
        return Err(malformed());
    }
    match req.action {
        RefereeAction::CardAnswer => {
            let card = req
                .card_id
                .as_deref()
                .ok_or_else(|| denied(StatusCode::CONFLICT, MessageCode::CardAnswerStale))?;
            let answer = req.answer.as_deref().ok_or_else(malformed)?;
            Ok(Some((card, answer)))
        }
        RefereeAction::DoneCriteria => match req.criteria {
            Some(_) => Ok(None),
            None => Err(malformed()),
        },
        RefereeAction::Cancel | RefereeAction::Stop | RefereeAction::Wake => Ok(None),
    }
}

fn malformed() -> CallerDenied {
    denied(StatusCode::BAD_REQUEST, MessageCode::RefereeMalformed)
}

fn already_answered(device: &str) -> CallerDenied {
    let params = Params::new().with_text("device", device);
    denied_with(
        StatusCode::CONFLICT,
        MessageCode::CardAlreadyAnswered,
        &params,
    )
}
