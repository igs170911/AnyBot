use axum::http::StatusCode;

use anybot_domain::card_waiting::{TaskRecordRef, card_is_waiting, card_relevant_kinds};
use anybot_messages::{MessageCode, Params};
use anybot_store_server::streams_card::{answered_by_in, task_card_records_in};

use crate::req_write::{CallerDenied, denied, denied_with, internal_sqlx, internal_text};

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

#[must_use]
pub fn already_answered(device: &str) -> CallerDenied {
    let params = Params::new().with_text("device", device);
    denied_with(
        StatusCode::CONFLICT,
        MessageCode::CardAlreadyAnswered,
        &params,
    )
}

pub async fn ensure_card_open(
    tx: &mut Tx<'_>,
    stream_key: &str,
    task_id: &str,
    card_id: &str,
) -> Result<(), CallerDenied> {
    let earlier = answered_by_in(tx, stream_key, card_id)
        .await
        .map_err(internal_sqlx)?;
    if let Some(device) = earlier {
        return Err(already_answered(&device));
    }
    let rows = task_card_records_in(tx, stream_key, task_id, &card_relevant_kinds())
        .await
        .map_err(internal_sqlx)?;
    let contents = rows
        .iter()
        .map(|row| serde_json::from_str(&row.content))
        .collect::<Result<Vec<serde_json::Value>, _>>()
        .map_err(|e| internal_text(e.to_string()))?;
    let history: Vec<TaskRecordRef<'_>> = rows
        .iter()
        .zip(&contents)
        .map(|(row, content)| TaskRecordRef {
            kind: &row.kind,
            content,
        })
        .collect();
    if card_is_waiting(card_id, &history) {
        return Ok(());
    }
    Err(denied(StatusCode::CONFLICT, MessageCode::CardNotWaiting))
}
