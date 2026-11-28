use anybot_protocol::CardAnswer;

use crate::answer_how::AnswerHow;

use crate::approvals::{PendingCard, resolve_approval_by};
use crate::error::TurnError;
use crate::runner::TurnRunner;

pub async fn release_for_yolo(
    runner: &TurnRunner,
    cards: Vec<PendingCard>,
) -> Result<Vec<PendingCard>, TurnError> {
    let mut kept = Vec::new();
    for card in cards {
        if !card.ticket.yolo_releasable() {
            kept.push(card);
            continue;
        }
        let ctx = runner.approval_context(card.task, card.turn);
        resolve_approval_by(
            runner,
            &ctx,
            &card,
            CardAnswer::AllowOnce,
            AnswerHow::yolo(),
        )
        .await?;
    }
    Ok(kept)
}
