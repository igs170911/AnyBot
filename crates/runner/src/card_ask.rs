use anybot_protocol::{AnswerOrigin, CardAnswer, KnownTaskEventKind, NotRunReason, ToolRequest};
use anybot_trust::AskTicket;
use anybot_trust::smart_timeout::SmartChoice;

use crate::action::OneAction;
use crate::answer_how::AnswerHow;
use crate::approvals::PendingCard;
use crate::card_wait::{CardWait, CardWaitOutcome, SmartAnswer};
use crate::error::TurnError;
use crate::record::{Appended, Halt};
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

const MILLIS_PER_SECOND: u64 = 1000;

pub(crate) async fn handle_ask(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    view: ActionView,
    ticket: AskTicket,
) -> Result<OneAction, TurnError> {
    let (card, shown) = crate::approvals::shown_card(runner, ctx, request, view, ticket)?;
    let stored = match runner.append_turn_unless_halted(ctx, shown).await? {
        Appended::Halted(halt) => {
            crate::action_record::record_not_run(runner, ctx, request, NotRunReason::Stopped)
                .await?;
            return Ok(OneAction::Halted(halt));
        }
        Appended::Stored(row) => row,
    };
    let wait = CardWait {
        shown_seq: crate::card_wait::task_seq_of(&stored),
        buttons: &card.buttons,
        smart: smart_answer(runner, &card)?,
    };
    let outcome = crate::card_wait::wait_for_card(runner, ctx, &wait).await?;
    settle(runner, ctx, card, outcome).await
}

async fn settle(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: PendingCard,
    outcome: CardWaitOutcome,
) -> Result<OneAction, TurnError> {
    match outcome {
        CardWaitOutcome::Answered(answer, origin) => {
            crate::approval_run::finish_in_turn(runner, ctx, &card, answer, how_of(origin)).await
        }
        CardWaitOutcome::SmartAnswered(smart) => {
            let how = AnswerHow::smart(smart.seconds);
            append_answer(runner, ctx, smart.answer, how.origin).await?;
            crate::approval_run::finish_in_turn(runner, ctx, &card, smart.answer, how).await
        }
        CardWaitOutcome::Halted(halt) => {
            if halt == Halt::Stopped {
                append_answer(runner, ctx, CardAnswer::Deny, AnswerOrigin::Stop).await?;
            }
            crate::action_record::record_not_run(runner, ctx, &card.request, NotRunReason::Stopped)
                .await?;
            Ok(OneAction::Halted(halt))
        }
        CardWaitOutcome::TimedOut => {
            crate::approvals::store_parked(runner, card)?;
            Ok(OneAction::TimedOut)
        }
    }
}

async fn append_answer(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    answer: CardAnswer,
    origin: AnswerOrigin,
) -> Result<(), TurnError> {
    runner
        .append_turn_event(ctx, KnownTaskEventKind::CardAnswered { answer, origin })
        .await
        .map(|_| ())
}

fn how_of(origin: AnswerOrigin) -> AnswerHow {
    match origin {
        AnswerOrigin::Hand => AnswerHow::hand(),
        AnswerOrigin::Stop => AnswerHow::stop(),
        AnswerOrigin::Auto => AnswerHow::yolo(),
    }
}

fn smart_answer(runner: &TurnRunner, card: &PendingCard) -> Result<Option<SmartAnswer>, TurnError> {
    let mdm_locked = runner.trust.modes()?.mdm_locks_direct;
    let answer = match card.ticket.smart_choice(mdm_locked) {
        SmartChoice::Never => return Ok(None),
        SmartChoice::AutoAllow => CardAnswer::AllowOnce,
        SmartChoice::AutoDeny => CardAnswer::Deny,
    };
    let seconds = runner.trust.smart_seconds()?;
    let wait_ms = u64::from(seconds).saturating_mul(MILLIS_PER_SECOND);
    Ok(Some(SmartAnswer {
        due_ms: card.parked_at_ms.saturating_add(wait_ms),
        answer,
        seconds,
    }))
}
