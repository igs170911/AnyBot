use std::collections::HashSet;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::StreamExt;
use tokio::sync::{broadcast, mpsc};

use anybot_bus::Notification;
use anybot_protocol::{MailHint, sse_event_line};
use anybot_store_server::mailbox;

use crate::api_streams_sse::SseQuery;
use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::signed;
use crate::device_notes::{device_of_mail, device_of_revoke};
use crate::req_write::{CallerDenied, internal_sqlx, internal_text, pool_of};

type Lines = mpsc::Sender<Result<Bytes, std::io::Error>>;

pub async fn notifications(
    State(state): State<AppState>,
    caller: Caller,
    Query(query): Query<SseQuery>,
) -> Result<Response, CallerDenied> {
    let is_device = matches!(
        caller.credential,
        CredentialKind::Computer | CredentialKind::Runner
    );
    if !is_device {
        return crate::api_streams_sse::notifications(State(state), caller, Query(query)).await;
    }
    let device = caller.device_id.clone();
    let rx = state.bus.subscribe();
    let inner =
        crate::api_streams_sse::notifications(State(state.clone()), caller, Query(query)).await?;
    let (parts, body) = inner.into_parts();
    let (tx, out) = mpsc::channel(128);
    tokio::spawn(merge(state, device, rx, body, tx));
    let stream = tokio_stream::wrappers::ReceiverStream::new(out);
    Ok(Response::from_parts(parts, Body::from_stream(stream)))
}

async fn merge(
    state: AppState,
    device: String,
    mut rx: broadcast::Receiver<Notification>,
    body: Body,
    tx: Lines,
) {
    let mut upstream = body.into_data_stream();
    let mut hinted: HashSet<String> = HashSet::new();
    if hint_pending(&state, &device, &mut hinted, &tx)
        .await
        .is_err()
    {
        return;
    }
    loop {
        tokio::select! {
            chunk = upstream.next() => match chunk {
                Some(Ok(bytes)) => {
                    if tx.send(Ok(bytes)).await.is_err() {
                        return;
                    }
                }
                Some(Err(_)) | None => return,
            },
            note = rx.recv() => {
                if !on_note(&state, &device, &mut hinted, &tx, note).await {
                    return;
                }
            }
        }
    }
}

async fn on_note(
    state: &AppState,
    device: &str,
    hinted: &mut HashSet<String>,
    tx: &Lines,
    note: Result<Notification, broadcast::error::RecvError>,
) -> bool {
    match note {
        Ok(note) if device_of_revoke(&note.kind) == Some(device) => false,
        Ok(note) if device_of_mail(&note.kind) == Some(device) => {
            hint_pending(state, device, hinted, tx).await.is_ok()
        }
        Ok(_other) => true,
        Err(broadcast::error::RecvError::Lagged(_skipped)) => {
            hint_pending(state, device, hinted, tx).await.is_ok()
        }
        Err(broadcast::error::RecvError::Closed) => false,
    }
}

async fn hint_pending(
    state: &AppState,
    device: &str,
    hinted: &mut HashSet<String>,
    tx: &Lines,
) -> Result<(), CallerDenied> {
    let pool = pool_of(state)?;
    let letters = mailbox::pending_for_device(pool, device)
        .await
        .map_err(internal_sqlx)?;
    for letter in letters {
        if !hinted.insert(letter.letter_id.clone()) {
            continue;
        }
        let hint = MailHint {
            letter_id: letter.letter_id.clone(),
            kind: letter.kind.clone(),
        };
        let json = serde_json::to_string(&hint).map_err(internal_text)?;
        let line = sse_event_line("mail", &json);
        tx.send(Ok(Bytes::from(line)))
            .await
            .map_err(|e| internal_text(e.to_string()))?;
        let now = signed(state.clock.now_ms().millis())?;
        mailbox::mark_delivered(pool, &letter.letter_id, now)
            .await
            .map_err(internal_sqlx)?;
    }
    Ok(())
}
