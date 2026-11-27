use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::{Value, json};

use anybot_domain::mailbox_letter::LetterKind;
use anybot_protocol::{AppendRequest, RefereeAction, RefereeRequest, selector_key};
use anybot_store_server::mailbox::{self, NewLetter};
use anybot_store_server::{Pool, computers};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::signed;
use crate::req_write::{CallerDenied, MAX_WRITE_BODY, internal_sqlx, internal_text, pool_of};

const KIND_CARD_SHOWN: &str = "card_shown";

async fn split(request: Request) -> Result<(axum::http::request::Parts, Vec<u8>), CallerDenied> {
    let (parts, body) = request.into_parts();
    let raw = axum::body::to_bytes(body, MAX_WRITE_BODY)
        .await
        .map_err(|e| internal_text(e.to_string()))?;
    Ok((parts, raw.to_vec()))
}

fn rebuilt(parts: axum::http::request::Parts, raw: Vec<u8>) -> Request {
    Request::from_parts(parts, Body::from(raw))
}

pub async fn append_noting_issuer(
    State(state): State<AppState>,
    caller: Caller,
    request: Request,
) -> Result<Response, CallerDenied> {
    let issuer = (caller.credential == CredentialKind::Computer).then(|| caller.device_id.clone());
    let account = caller.account.clone();
    let (parts, raw) = split(request).await?;
    let parsed = serde_json::from_slice::<AppendRequest>(&raw);
    let answer =
        crate::api_streams_append::append(State(state.clone()), caller, rebuilt(parts, raw))
            .await?;
    if let (Some(device), Ok(request), StatusCode::OK) = (issuer, parsed, answer.status()) {
        note_cards(&state, &device, &account, &request).await?;
    }
    Ok(answer)
}

async fn note_cards(
    state: &AppState,
    device: &str,
    account: &str,
    request: &AppendRequest,
) -> Result<(), CallerDenied> {
    let pool = pool_of(state)?;
    let stream = selector_key(&request.stream, account);
    let now = signed(state.clock.now_ms().millis())?;
    for item in request
        .items
        .iter()
        .filter(|item| item.kind == KIND_CARD_SHOWN)
    {
        let Some(card) = item.content.get("card").and_then(Value::as_str) else {
            continue;
        };
        mailbox::note_issuer(pool, &stream, card, device, now)
            .await
            .map_err(internal_sqlx)?;
    }
    Ok(())
}

pub async fn referee_with_letter(
    State(state): State<AppState>,
    caller: Caller,
    request: Request,
) -> Result<Response, CallerDenied> {
    let account = caller.account.clone();
    let (parts, raw) = split(request).await?;
    let parsed = serde_json::from_slice::<RefereeRequest>(&raw);
    let answer =
        crate::api_referee::referee(State(state.clone()), caller, rebuilt(parts, raw)).await?;
    if let (Ok(request), StatusCode::OK) = (parsed, answer.status())
        && request.action == RefereeAction::CardAnswer
    {
        post_answer_letter(&state, &account, &request).await?;
    }
    Ok(answer)
}

async fn live_issuer(
    pool: &Pool,
    account: &str,
    stream: &str,
    card: &str,
) -> Result<Option<String>, CallerDenied> {
    let issuer = mailbox::issuer_of(pool, stream, card)
        .await
        .map_err(internal_sqlx)?;
    let Some(device) = issuer else {
        return Ok(None);
    };
    let row = computers::find_by_device(pool, &device)
        .await
        .map_err(internal_sqlx)?;
    Ok(row
        .filter(|row| row.account == account && row.removed_at_ms.is_none())
        .map(|row| row.device_id))
}

async fn post_answer_letter(
    state: &AppState,
    account: &str,
    request: &RefereeRequest,
) -> Result<(), CallerDenied> {
    let (Some(card), Some(answer)) = (request.card_id.as_deref(), request.answer.as_deref()) else {
        return Ok(());
    };
    let pool = pool_of(state)?;
    let stream = selector_key(&request.stream, account);
    let Some(device) = live_issuer(pool, account, &stream, card).await? else {
        return Ok(());
    };
    let content = json!({
        "stream": stream,
        "task_id": request.task_id,
        "card_id": card,
        "answer": answer,
    })
    .to_string();
    let letter_id = format!("card:{stream}:{card}");
    let fresh = NewLetter {
        letter_id: &letter_id,
        account,
        kind: LetterKind::CardAnswer.code(),
        device_id: &device,
        content: &content,
        now_ms: signed(state.clock.now_ms().millis())?,
    };
    let added = mailbox::insert(pool, &fresh).await.map_err(internal_sqlx)?;
    if added {
        crate::device_notes::notify_mail(state, &device);
    }
    Ok(())
}
