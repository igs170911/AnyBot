use anybot_protocol::{CardAnswer, KnownTaskEventKind, NotRunReason, ToolRequest, TurnInitiator};
use anybot_trust::{ActionRun, AskTicket, GateAnswer};

use std::sync::Arc;

use anybot_foundation::{TaskId, TurnId};

use crate::action::OneAction;
use crate::error::TurnError;
use crate::record::Halt;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

#[derive(Debug)]
pub struct PendingCard {
    pub digest: String,
    pub tool: String,
    pub computer: String,
    pub actions: Vec<String>,
    ticket: AskTicket,
    request: ToolRequest,
    view: ActionView,
}

impl PendingCard {
    #[must_use]
    pub fn is_for(&self, digest: &str) -> bool {
        self.digest == digest
    }
}

pub(crate) fn store_parked(
    runner: &TurnRunner,
    request: &ToolRequest,
    view: ActionView,
    ticket: AskTicket,
) -> Result<(), TurnError> {
    let card = PendingCard {
        digest: ticket.card_digest(),
        tool: request.tool.clone(),
        computer: ticket.computer_name().to_owned(),
        actions: ticket.card().actions.clone(),
        ticket,
        request: request.clone(),
        view,
    };
    let Ok(mut guard) = runner.parked.lock() else {
        return Err(TurnError::ParkedBusy);
    };
    guard.push(card);
    Ok(())
}

pub(crate) async fn finish_in_turn_approval(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    ticket: &AskTicket,
    view: &ActionView,
    answer: CardAnswer,
) -> Result<OneAction, TurnError> {
    if let CardAnswer::AllowOnce | CardAnswer::AllowConversation | CardAnswer::AlwaysAllow = answer
        && let Some(halt) = crate::halt::check_halt(runner, ctx).await?
    {
        crate::action_record::record_not_run(runner, ctx, request, NotRunReason::Stopped).await?;
        return Ok(OneAction::Halted(halt));
    }
    let computer = runner.gated_ref();
    let gate = gate_answer_of(answer);
    let run = anybot_trust::answer_ask(ticket, gate, &*computer, view, &*runner.clock);
    match answer {
        CardAnswer::Deny => {
            crate::action_record::record_denied(runner, ctx, request, &run).await?;
            record_refusal(runner, ctx, request, &run, view).await?;
            Ok(OneAction::Refused)
        }
        CardAnswer::AllowOnce | CardAnswer::AllowConversation | CardAnswer::AlwaysAllow => {
            match record_approval_run(runner, ctx, request, &run, view).await? {
                Some(halt) => Ok(OneAction::Halted(halt)),
                None => Ok(OneAction::Done),
            }
        }
    }
}

pub(crate) async fn record_ask_summary(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
) -> Result<(), TurnError> {
    let text = view.render(&run.model)?;
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ToolCompleted {
                call_id: request.call_id,
                summary: text,
                raw_id: None,
            },
        )
        .await
        .map(|_| ())
}

pub(crate) fn take_parked(runner: &TurnRunner) -> Vec<PendingCard> {
    let Ok(mut guard) = runner.parked.lock() else {
        return Vec::new();
    };
    std::mem::take(&mut *guard)
}

pub async fn resolve_approval(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
) -> Result<(), TurnError> {
    let gate = gate_answer_of(answer);
    runner
        .append_turn_event(ctx, KnownTaskEventKind::CardAnswered { answer })
        .await?;
    let computer = runner.gated_ref();
    let run = anybot_trust::answer_ask(&card.ticket, gate, &*computer, &card.view, &*runner.clock);
    match answer {
        CardAnswer::Deny => {
            crate::action_record::record_denied(runner, ctx, &card.request, &run).await?;
            record_refusal(runner, ctx, &card.request, &run, &card.view).await
        }
        CardAnswer::AllowOnce | CardAnswer::AllowConversation | CardAnswer::AlwaysAllow => {
            record_approval_run(runner, ctx, &card.request, &run, &card.view)
                .await
                .map(|_| ())
        }
    }
}

async fn record_approval_run(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
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
    crate::action_record::record_result(runner, ctx, request, run, view).await?;
    let ids = crate::action_record::record_evidence(runner, ctx, &run.evidence).await?;
    crate::action_record::record_verified(runner, ctx, request.call_id, run, ids).await?;
    Ok(None)
}

fn gate_answer_of(answer: CardAnswer) -> GateAnswer {
    match answer {
        CardAnswer::AllowOnce | CardAnswer::AllowConversation | CardAnswer::AlwaysAllow => {
            GateAnswer::Allow
        }
        CardAnswer::Deny => GateAnswer::Deny,
    }
}

async fn record_refusal(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
) -> Result<(), TurnError> {
    let refusal = view.render(&anybot_trust::OutcomeText::of(
        anybot_trust::CODE_GATE_ASK_DECLINED,
        Vec::new(),
    ))?;
    let text = view.render(&anybot_trust::OutcomeText::of(
        "gate-denied",
        vec![
            ("computer".to_owned(), view.computer_name().to_owned()),
            ("rule".to_owned(), run.gate.rule.clone()),
            ("refusal".to_owned(), refusal),
        ],
    ))?;
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ToolCompleted {
                call_id: request.call_id,
                summary: text,
                raw_id: None,
            },
        )
        .await
        .map(|_| ())
}

pub(crate) fn first_action(ticket: &AskTicket) -> String {
    ticket
        .card()
        .actions
        .first()
        .cloned()
        .unwrap_or_else(|| "approval".to_owned())
}

impl TurnRunner {
    pub fn approval_context(&self, task: TaskId, turn: u32) -> TurnCtx {
        TurnCtx {
            task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            turn_started: 0,
            initiator: TurnInitiator::Verification,
            evidence: tokio::sync::Mutex::new(1_000),
            active_model: std::sync::Mutex::new(None),
        }
    }

    #[must_use]
    pub fn last_ctx(&self) -> Option<TurnCtx> {
        let Ok(guard) = self.last_ctx.lock() else {
            return None;
        };
        guard.clone()
    }

    pub fn bump_evidence(ctx: &TurnCtx) {
        let Ok(mut guard) = ctx.evidence.try_lock() else {
            return;
        };
        *guard = guard.saturating_add(1_000);
    }

    #[must_use]
    pub fn gated_ref(&self) -> Arc<dyn anybot_trust::GatedComputer> {
        Arc::clone(&self.gated)
    }
}
