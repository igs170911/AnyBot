use axum::http::StatusCode;
use serde_json::{Value, json};

use anybot_domain::record_actions::{
    KIND_RECORD_CLEAR, KIND_RECORD_DELETE, KIND_RECORD_PURGE_PERMANENT, KIND_RECORD_RESTORE,
    is_chat_kind, is_hideable_action_kind, restore_window_open, scrub_chat_content,
};
use anybot_messages::MessageCode;
use anybot_protocol::{RecordAction, RecordActionRequest};
use anybot_store_server::streams::RecordRow;
use anybot_store_server::streams_action;

use crate::req_write::{CallerDenied, denied, internal_sqlx, internal_text};

pub struct ActionPlan {
    pub kind: &'static str,
    pub content: String,
    pub purge: Option<(i64, i64)>,
}

#[must_use]
pub fn action_device_id(key: &str) -> String {
    format!("action:{key}")
}

pub async fn plan_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stream_key: &str,
    req: &RecordActionRequest,
    max: i64,
    now: i64,
) -> Result<ActionPlan, CallerDenied> {
    if req.action == RecordAction::Clear {
        let content = json!({"through_seq": max}).to_string();
        return Ok(ActionPlan {
            kind: KIND_RECORD_CLEAR,
            content,
            purge: None,
        });
    }
    let target = req.target_seq.ok_or_else(|| {
        denied(
            StatusCode::BAD_REQUEST,
            MessageCode::RecordActionTargetRequired,
        )
    })?;
    let row = target_row(tx, stream_key, target).await?;
    match req.action {
        RecordAction::Clear => Err(invalid()),
        RecordAction::Delete => plan_delete(&row, target),
        RecordAction::Restore => plan_restore(&row, target, now),
        RecordAction::PurgePermanent => plan_purge(&row, target),
    }
}

fn plan_delete(row: &RecordRow, target: i64) -> Result<ActionPlan, CallerDenied> {
    if !is_chat_kind(&row.kind) || row.purged {
        return Err(invalid());
    }
    Ok(targeted(KIND_RECORD_DELETE, target, None))
}

fn plan_restore(row: &RecordRow, target: i64, now: i64) -> Result<ActionPlan, CallerDenied> {
    if !is_hideable_action_kind(&row.kind) {
        return Err(invalid());
    }
    match restore_window_open(row.created_at_ms, now) {
        Some(true) => Ok(targeted(KIND_RECORD_RESTORE, target, None)),
        Some(false) => Err(denied(
            StatusCode::CONFLICT,
            MessageCode::RecordRestoreExpired,
        )),
        None => Err(internal_text("restore window overflowed")),
    }
}

fn plan_purge(row: &RecordRow, target: i64) -> Result<ActionPlan, CallerDenied> {
    if !is_hideable_action_kind(&row.kind) {
        return Err(invalid());
    }
    let range = hidden_range(row)?;
    Ok(targeted(KIND_RECORD_PURGE_PERMANENT, target, Some(range)))
}

fn hidden_range(row: &RecordRow) -> Result<(i64, i64), CallerDenied> {
    let content: Value =
        serde_json::from_str(&row.content).map_err(|e| internal_text(e.to_string()))?;
    if row.kind == KIND_RECORD_DELETE {
        let target = field_i64(&content, "target_seq")?;
        return Ok((target, target));
    }
    let through = field_i64(&content, "through_seq")?;
    Ok((1, through))
}

fn field_i64(content: &Value, field: &str) -> Result<i64, CallerDenied> {
    content
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| internal_text(format!("action record missing {field}")))
}

fn targeted(kind: &'static str, target: i64, purge: Option<(i64, i64)>) -> ActionPlan {
    ActionPlan {
        kind,
        content: json!({"target_seq": target}).to_string(),
        purge,
    }
}

async fn target_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stream_key: &str,
    target: i64,
) -> Result<RecordRow, CallerDenied> {
    let found = streams_action::record_at_in(tx, stream_key, target)
        .await
        .map_err(internal_sqlx)?;
    found.ok_or_else(|| {
        denied(
            StatusCode::NOT_FOUND,
            MessageCode::RecordActionTargetMissing,
        )
    })
}

fn invalid() -> CallerDenied {
    denied(
        StatusCode::BAD_REQUEST,
        MessageCode::RecordActionTargetInvalid,
    )
}

pub async fn purge_range(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stream_key: &str,
    low: i64,
    high: i64,
) -> Result<(), CallerDenied> {
    let rows = streams_action::chat_rows_between_in(tx, stream_key, low, high)
        .await
        .map_err(internal_sqlx)?;
    for row in rows {
        let content: Value =
            serde_json::from_str(&row.content).map_err(|e| internal_text(e.to_string()))?;
        let scrubbed = scrub_chat_content(&row.kind, &content).to_string();
        streams_action::mark_purged_in(tx, stream_key, row.seq, &scrubbed)
            .await
            .map_err(internal_sqlx)?;
    }
    Ok(())
}
