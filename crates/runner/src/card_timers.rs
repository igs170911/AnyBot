use anybot_protocol::CardAnswer;
use anybot_trust::smart_timeout::SmartChoice;

use crate::answer_how::AnswerHow;
use crate::approvals::{PendingCard, resolve_approval_by};
use crate::error::TurnError;
use crate::runner::TurnRunner;

const MILLIS_PER_SECOND: u64 = 1000;

pub fn next_expiry_ms(
    runner: &TurnRunner,
    cards: &[PendingCard],
) -> Result<Option<u64>, TurnError> {
    let seconds = runner.trust.smart_seconds()?;
    let wait_ms = u64::from(seconds).saturating_mul(MILLIS_PER_SECOND);
    let mdm_locked = runner.trust.modes()?.mdm_locks_direct;
    Ok(cards
        .iter()
        .filter(|card| card.ticket.smart_choice(mdm_locked) != SmartChoice::Never)
        .map(|card| card.parked_at_ms.saturating_add(wait_ms))
        .min())
}

pub async fn answer_expired_cards(
    runner: &TurnRunner,
    cards: Vec<PendingCard>,
) -> Result<Vec<PendingCard>, TurnError> {
    let seconds = runner.trust.smart_seconds()?;
    let wait_ms = u64::from(seconds).saturating_mul(MILLIS_PER_SECOND);
    let mdm_locked = runner.trust.modes()?.mdm_locks_direct;
    let mut kept = Vec::new();
    for card in cards {
        let due_ms = card.parked_at_ms.saturating_add(wait_ms);
        let choice = card.ticket.smart_choice(mdm_locked);
        if runner.clock.now_ms().millis() < due_ms || choice == SmartChoice::Never {
            kept.push(card);
            continue;
        }
        let answer = if choice == SmartChoice::AutoAllow {
            CardAnswer::AllowOnce
        } else {
            CardAnswer::Deny
        };
        let ctx = runner.approval_context(card.task, card.turn);
        resolve_approval_by(runner, &ctx, &card, answer, AnswerHow::smart(seconds)).await?;
    }
    Ok(kept)
}
