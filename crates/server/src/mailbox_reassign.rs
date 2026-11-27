use serde_json::{Value, json};

use anybot_domain::mailbox_route::ReassignReason;
use anybot_store_server::mailbox::{self, LetterRow, NewLetter};
use anybot_store_server::{Pool, computers, runner_device, streams};

use crate::app::AppState;
use crate::computer_view::signed;
use crate::req_write::{CallerDenied, internal_detail, internal_sqlx, internal_text};

pub(crate) const KIND_MAILBOX_REASSIGNED: &str = "mailbox_reassigned";

pub(crate) async fn bot_computer_id(pool: &Pool) -> Result<String, CallerDenied> {
    let row = runner_device::find(pool).await.map_err(internal_sqlx)?;
    row.map(|row| row.device_id)
        .ok_or_else(|| internal_detail("the Bot computer is missing"))
}

fn reason_code(reason: ReassignReason) -> &'static str {
    match reason {
        ReassignReason::Offline => "mailbox-reassign-offline",
        ReassignReason::DelegateOff => "mailbox-reassign-delegate-off",
    }
}

pub(crate) async fn computer_name(pool: &Pool, device_id: &str) -> Result<String, CallerDenied> {
    let row = computers::find_by_device(pool, device_id)
        .await
        .map_err(internal_sqlx)?;
    Ok(row.map(|row| row.name).unwrap_or_default())
}

pub(crate) struct Settled<'a> {
    pub account: &'a str,
    pub bot_id: &'a str,
    pub from_device: &'a str,
    pub new_letter: &'a str,
    pub reason: ReassignReason,
}

pub(crate) async fn write_line(
    state: &AppState,
    pool: &Pool,
    settled: &Settled<'_>,
) -> Result<(), CallerDenied> {
    let now = signed(state.clock.now_ms().millis())?;
    let key = format!("bot/{}", settled.bot_id);
    streams::ensure_stream(
        pool,
        &key,
        "bot",
        settled.account,
        Some(settled.bot_id),
        now,
    )
    .await
    .map_err(internal_sqlx)?;
    let name = computer_name(pool, settled.from_device).await?;
    let content = json!({
        "code": reason_code(settled.reason),
        "params": {"computer": name},
        "from": settled.from_device,
        "letter": settled.new_letter,
    })
    .to_string();
    let record_id = format!("mailbox:{}", settled.new_letter);
    let spec = streams::InsertSpec {
        device_id: &record_id,
        kind: KIND_MAILBOX_REASSIGNED,
        content: &content,
        created_at_ms: now,
        offline_sent: false,
        task_id: None,
    };
    let seq = streams::insert_record(pool, &key, &spec)
        .await
        .map_err(internal_sqlx)?;
    crate::api_streams_action::publish_note(state, &key, seq);
    Ok(())
}

pub(crate) async fn reassign(
    state: &AppState,
    pool: &Pool,
    letter: &LetterRow,
    reason: ReassignReason,
) -> Result<bool, CallerDenied> {
    let runner = bot_computer_id(pool).await?;
    let new_letter = uuid::Uuid::now_v7().to_string();
    let now = signed(state.clock.now_ms().millis())?;
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    let moved =
        mailbox::mark_reassigned_in(&mut tx, &letter.letter_id, &new_letter, reason.code(), now)
            .await
            .map_err(internal_sqlx)?;
    if !moved {
        return Ok(false);
    }
    let fresh = NewLetter {
        letter_id: &new_letter,
        account: &letter.account,
        kind: &letter.kind,
        device_id: &runner,
        content: &letter.content,
        now_ms: now,
    };
    mailbox::insert_in(&mut tx, &fresh)
        .await
        .map_err(internal_sqlx)?;
    tx.commit().await.map_err(internal_sqlx)?;
    announce_move(state, pool, letter, &new_letter, reason).await?;
    crate::device_notes::notify_mail(state, &runner);
    Ok(true)
}

async fn announce_move(
    state: &AppState,
    pool: &Pool,
    letter: &LetterRow,
    new_letter: &str,
    reason: ReassignReason,
) -> Result<(), CallerDenied> {
    let content: Value = serde_json::from_str(&letter.content).map_err(internal_text)?;
    let Some(bot_id) = content.get("bot").and_then(Value::as_str) else {
        return Ok(());
    };
    let settled = Settled {
        account: &letter.account,
        bot_id,
        from_device: &letter.device_id,
        new_letter,
        reason,
    };
    write_line(state, pool, &settled).await
}
