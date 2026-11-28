use anybot_protocol::{CardAnswer, KnownTaskEventKind, NotRunReason, ToolRequest};
use anybot_trust::{ActionRun, GateAnswer};

use crate::action::OneAction;
use crate::answer_how::AnswerHow;
use crate::approvals::PendingCard;
use crate::error::TurnError;
use crate::record::Halt;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) async fn finish_in_turn(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
    how: AnswerHow,
) -> Result<OneAction, TurnError> {
    if answer != CardAnswer::Deny
        && let Some(halt) = crate::halt::check_halt(runner, ctx).await?
    {
        crate::action_record::record_not_run(runner, ctx, &card.request, NotRunReason::Stopped)
            .await?;
        return Ok(OneAction::Halted(halt));
    }
    apply_answer(runner, ctx, card, answer, how).await
}

pub(crate) async fn apply_answer(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
    how: AnswerHow,
) -> Result<OneAction, TurnError> {
    crate::reject_memory::note_answer(runner, &card.request, answer, how.origin)?;
    crate::answer_effects::apply(runner, card, answer)?;
    if card.ticket.switch_info().is_some() && answer != CardAnswer::Deny {
        return crate::switch_answer::continue_after_switch(runner, ctx, card).await;
    }
    execute_answer(runner, ctx, card, answer, how).await
}

async fn execute_answer(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
    how: AnswerHow,
) -> Result<OneAction, TurnError> {
    let gate = gate_answer_of(answer);
    let computer = runner.gated_ref();
    let run = anybot_trust::answer_ask_by(
        &card.ticket,
        gate,
        how.source,
        &*computer,
        &card.view,
        &*runner.clock,
    );
    if answer == CardAnswer::Deny {
        crate::action_record::record_denied(runner, ctx, &card.request, &run).await?;
        record_refusal(runner, ctx, card, &run).await?;
        return Ok(OneAction::Refused);
    }
    match record_approval_run(runner, ctx, &card.request, &run, &card.view, how).await? {
        Some(halt) => Ok(OneAction::Halted(halt)),
        None => Ok(OneAction::Done),
    }
}

async fn record_approval_run(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
    how: AnswerHow,
) -> Result<Option<Halt>, TurnError> {
    crate::action_record::record_gate(runner, ctx, request.call_id, &run.gate).await?;
    if let Some(halt) =
        crate::action_record::record_intent_start(runner, ctx, request, &run.gate).await?
    {
        crate::action_record::record_not_run(runner, ctx, request, NotRunReason::Stopped).await?;
        return Ok(Some(halt));
    }
    if let Some(word) = crate::action::model_word(&run.summary.status) {
        let reason = crate::action::not_run_reason_of(&word);
        crate::action_record::record_not_run(runner, ctx, request, reason).await?;
        return Ok(None);
    }
    let note = smart_note(view, how)?;
    crate::action_events::record_result_noted(runner, ctx, request, run, view, note).await?;
    let ids = crate::action_record::record_evidence(runner, ctx, &run.evidence).await?;
    crate::action_record::record_verified(runner, ctx, request.call_id, run, ids).await?;
    Ok(None)
}

fn smart_note(view: &ActionView, how: AnswerHow) -> Result<Option<String>, TurnError> {
    let Some(seconds) = how.smart_wait_seconds else {
        return Ok(None);
    };
    let text = anybot_trust::OutcomeText::of(
        "gate-smart-auto-allowed",
        vec![("seconds".to_owned(), seconds.to_string())],
    );
    view.render(&text).map(Some)
}

fn gate_answer_of(answer: CardAnswer) -> GateAnswer {
    match answer {
        CardAnswer::AllowOnce
        | CardAnswer::AllowConversation
        | CardAnswer::AlwaysAllow
        | CardAnswer::DirectMode => GateAnswer::Allow,
        CardAnswer::Deny => GateAnswer::Deny,
    }
}

async fn record_refusal(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    run: &ActionRun,
) -> Result<(), TurnError> {
    let view = &card.view;
    let text = match card.ticket.decline_text() {
        Some(declined) => view.render(&declined)?,
        None => {
            let refusal = view.render(&anybot_trust::OutcomeText::of(
                anybot_trust::CODE_GATE_ASK_DECLINED,
                Vec::new(),
            ))?;
            view.render(&anybot_trust::OutcomeText::of(
                "gate-denied",
                vec![
                    ("computer".to_owned(), view.computer_name().to_owned()),
                    ("rule".to_owned(), run.gate.rule.clone()),
                    ("refusal".to_owned(), refusal),
                ],
            ))?
        }
    };
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ToolCompleted {
                call_id: card.request.call_id,
                summary: text,
                raw_id: None,
            },
        )
        .await
        .map(|_| ())
}
