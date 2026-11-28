use anybot_protocol::{CardAnswer, KnownTaskEventKind, ToolRequest};
use anybot_trust::{ActionRun, AskTicket, GateAnswer};

use std::sync::Arc;

use anybot_foundation::{TaskId, TurnId};

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

#[derive(Debug)]
pub struct PendingCard {
    pub digest: String,
    pub tool: String,
    pub computer: String,
    pub actions: Vec<String>,
    pub(crate) ticket: AskTicket,
    pub(crate) request: ToolRequest,
    pub(crate) view: ActionView,
}

impl PendingCard {
    #[must_use]
    pub fn is_for(&self, digest: &str) -> bool {
        self.digest == digest
    }
}

pub(crate) async fn park_approval(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    view: ActionView,
    ticket: AskTicket,
) -> Result<(), TurnError> {
    let card_view = ticket.card_view()?;
    let digest = ticket.card_digest();
    let kind = KnownTaskEventKind::CardShown {
        action: card_view.action,
        card: digest.clone(),
        kind: card_view.kind,
        buttons: card_view.buttons,
        reason: card_view.reason,
        always_label: card_view.always_label,
        notes: card_view.notes,
        classes: card_view.classes,
    };
    runner.append_task_event(&ctx.task, kind).await?;
    let card = PendingCard {
        digest,
        tool: request.tool.clone(),
        computer: ticket.computer_name().to_owned(),
        actions: card_view.actions,
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

pub(crate) async fn record_ask_summary(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
) -> Result<(), TurnError> {
    let text = view.render(&run.model)?;
    runner
        .append_task_event(
            &ctx.task,
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

pub(crate) fn has_parked(runner: &TurnRunner) -> bool {
    let Ok(guard) = runner.parked.lock() else {
        return false;
    };
    !guard.is_empty()
}

pub async fn resolve_approval(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
) -> Result<(), TurnError> {
    let existing = runner.record.task_events(ctx.task).await?;
    let next = max_evidence_short(&existing).saturating_add(1);
    ctx.advance_evidence_to(next);
    runner
        .append_task_event(&ctx.task, KnownTaskEventKind::CardAnswered { answer })
        .await?;
    let result = if card.ticket.switch_info().is_some() && answer != CardAnswer::Deny {
        crate::switch_answer::continue_after_switch(runner, ctx, card).await
    } else {
        execute_answer(runner, ctx, card, answer).await
    };
    refresh_last_ctx(runner, ctx);
    result
}

async fn execute_answer(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
) -> Result<(), TurnError> {
    let gate = gate_answer_of(answer);
    let computer = runner.gated_ref();
    let run = anybot_trust::answer_ask(&card.ticket, gate, &*computer, &card.view, &*runner.clock);
    match answer {
        CardAnswer::Deny => {
            crate::action_record::record_denied(runner, ctx, &card.request, &run).await?;
            record_refusal(runner, ctx, card, &run).await
        }
        CardAnswer::AllowOnce
        | CardAnswer::AllowConversation
        | CardAnswer::AlwaysAllow
        | CardAnswer::DirectMode => {
            record_approval_run(runner, ctx, &card.request, &run, &card.view).await
        }
    }
}

fn refresh_last_ctx(runner: &TurnRunner, ctx: &TurnCtx) {
    if let Ok(mut guard) = runner.last_ctx.lock()
        && guard.as_ref().is_some_and(|prev| prev.task == ctx.task)
    {
        *guard = Some(ctx.clone());
    }
}

async fn record_approval_run(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
    view: &ActionView,
) -> Result<(), TurnError> {
    crate::action_record::record_gate(runner, ctx, request.call_id, &run.gate).await?;
    crate::action_record::record_intent_start(runner, ctx, request, &run.gate).await?;
    crate::action_record::record_result(runner, ctx, request, run, view).await?;
    let ids = crate::action_record::record_evidence(runner, ctx, &run.evidence).await?;
    crate::action_record::record_verified(runner, ctx, request.call_id, run, ids).await
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
    let text = match card.ticket.decline_text() {
        Some(declined) => card.view.render(&declined)?,
        None => card.view.render(&anybot_trust::OutcomeText::of(
            "gate-denied",
            vec![
                ("computer".to_owned(), card.view.computer_name().to_owned()),
                ("rule".to_owned(), run.gate.rule.clone()),
                (
                    "refusal".to_owned(),
                    anybot_trust::CODE_GATE_ASK_DECLINED.to_owned(),
                ),
            ],
        ))?,
    };
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolCompleted {
                call_id: card.request.call_id,
                summary: text,
                raw_id: None,
            },
        )
        .await
        .map(|_| ())
}

impl TurnRunner {
    pub fn take_parked_cards(&self) -> Vec<PendingCard> {
        take_parked(self)
    }

    pub fn approval_context(&self, task: TaskId, turn: u32) -> TurnCtx {
        let next = self
            .last_ctx()
            .filter(|ctx| ctx.task == task)
            .map_or(1, |ctx| ctx.peek_evidence());
        TurnCtx {
            task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            evidence: tokio::sync::Mutex::new(next),
        }
    }

    #[must_use]
    pub fn last_ctx(&self) -> Option<TurnCtx> {
        let Ok(guard) = self.last_ctx.lock() else {
            return None;
        };
        guard.clone()
    }

    #[must_use]
    pub fn gated_ref(&self) -> Arc<dyn anybot_trust::GatedComputer> {
        Arc::clone(&self.gated)
    }
}

fn max_evidence_short(events: &[anybot_protocol::TaskEvent]) -> u32 {
    events
        .iter()
        .filter_map(|event| {
            let anybot_protocol::TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded {
                record,
            }) = &event.kind
            else {
                return None;
            };
            Some(record.short.number())
        })
        .max()
        .unwrap_or(0)
}
