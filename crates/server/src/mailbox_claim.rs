use axum::http::StatusCode;

use anybot_domain::mailbox_letter::{
    ClaimRefusal, LetterKind, LetterState, claim_expired, refusal_for,
};
use anybot_domain::mailbox_route::ReassignReason;
use anybot_messages::MessageCode;
use anybot_protocol::ClaimReply;
use anybot_store_server::Pool;
use anybot_store_server::mailbox::{self, LetterRow};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::{signed, unsigned};
use crate::mailbox_reassign::{bot_computer_id, reassign};
use crate::req_write::{CallerDenied, denied, internal_sqlx, internal_text};

fn refusal_code(refusal: ClaimRefusal) -> MessageCode {
    match refusal {
        ClaimRefusal::Reassigned => MessageCode::MailboxClaimReassigned,
        ClaimRefusal::TakenByOther => MessageCode::MailboxClaimTaken,
        ClaimRefusal::AlreadyYours => MessageCode::MailboxClaimYours,
        ClaimRefusal::Retracted => MessageCode::MailboxClaimRetracted,
    }
}

fn unknown() -> CallerDenied {
    denied(StatusCode::NOT_FOUND, MessageCode::MailboxLetterUnknown)
}

fn state_of(letter: &LetterRow) -> Result<LetterState, CallerDenied> {
    LetterState::from_code(&letter.state).ok_or_else(|| internal_text("unknown letter state"))
}

fn visible_to(caller: &Caller, letter: &LetterRow) -> bool {
    caller.credential == CredentialKind::Runner || letter.account == caller.account
}

async fn find_visible(
    pool: &Pool,
    caller: &Caller,
    letter_id: &str,
) -> Result<LetterRow, CallerDenied> {
    let found = mailbox::find(pool, letter_id)
        .await
        .map_err(internal_sqlx)?;
    found
        .filter(|letter| visible_to(caller, letter))
        .ok_or_else(unknown)
}

async fn lapse_if_unclaimed(
    state: &AppState,
    pool: &Pool,
    letter: &LetterRow,
) -> Result<(), CallerDenied> {
    let late = letter.kind == LetterKind::Round.code()
        && letter.state == LetterState::Pending.code()
        && claim_expired(
            unsigned(letter.created_at_ms)?,
            letter.delivered_at_ms.map(unsigned).transpose()?,
            state.clock.now_ms().millis(),
        );
    if late && letter.device_id != bot_computer_id(pool).await? {
        reassign(state, pool, letter, ReassignReason::Offline).await?;
    }
    Ok(())
}

pub(crate) async fn claim(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    letter_id: &str,
) -> Result<ClaimReply, CallerDenied> {
    let first = find_visible(pool, caller, letter_id).await?;
    lapse_if_unclaimed(state, pool, &first).await?;
    let now = signed(state.clock.now_ms().millis())?;
    let won = mailbox::claim(pool, letter_id, &caller.device_id, now)
        .await
        .map_err(internal_sqlx)?;
    if won {
        return reply_of(&first);
    }
    let letter = find_visible(pool, caller, letter_id).await?;
    let refusal = refusal_for(
        state_of(&letter)?,
        letter.claimed_by.as_deref(),
        &caller.device_id,
    );
    Err(refusal.map_or_else(unknown, |refusal| {
        denied(StatusCode::CONFLICT, refusal_code(refusal))
    }))
}

fn reply_of(letter: &LetterRow) -> Result<ClaimReply, CallerDenied> {
    let content = serde_json::from_str(&letter.content).map_err(internal_text)?;
    Ok(ClaimReply {
        letter_id: letter.letter_id.clone(),
        kind: letter.kind.clone(),
        content,
    })
}
