use anybot_protocol::{CardAnswer, KnownTaskEventKind, ToolRequest, TurnInitiator};
use anybot_trust::{ActionRun, AskTicket};

use std::sync::Arc;

use anybot_foundation::{TaskId, TurnId};

use crate::answer_how::AnswerHow;
use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

#[derive(Debug)]
pub struct PendingCard {
    pub digest: String,
    pub(crate) task: TaskId,
    pub(crate) turn: u32,
    pub(crate) parked_at_ms: u64,
    pub tool: String,
    pub computer: String,
    pub actions: Vec<String>,
    pub(crate) buttons: Vec<anybot_protocol::CardButton>,
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

pub(crate) fn shown_card(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    view: ActionView,
    ticket: AskTicket,
) -> Result<(PendingCard, KnownTaskEventKind), TurnError> {
    let card_view = ticket.card_view()?;
    let digest = ticket.card_digest();
    let shown = KnownTaskEventKind::CardShown {
        action: card_view.action,
        card: digest.clone(),
        kind: card_view.kind,
        resume: None,
        buttons: card_view.buttons.clone(),
        reason: card_view.reason,
        always_label: card_view.always_label,
        notes: card_view.notes,
        classes: card_view.classes,
    };
    let card = PendingCard {
        digest,
        task: ctx.task,
        turn: ctx.turn,
        parked_at_ms: runner.clock.now_ms().millis(),
        tool: request.tool.clone(),
        computer: ticket.computer_name().to_owned(),
        actions: card_view.actions,
        buttons: card_view.buttons,
        ticket,
        request: request.clone(),
        view,
    };
    Ok((card, shown))
}

pub(crate) fn store_parked(runner: &TurnRunner, card: PendingCard) -> Result<(), TurnError> {
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
    resolve_approval_by(runner, ctx, card, answer, AnswerHow::hand()).await
}

pub async fn resolve_approval_by(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
    answer: CardAnswer,
    how: AnswerHow,
) -> Result<(), TurnError> {
    crate::answer_effects::ensure_offered(card, answer)?;
    let existing = runner.record.task_events(ctx.task).await?;
    let next = max_evidence_short(&existing).saturating_add(1);
    ctx.advance_evidence_to(next);
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::CardAnswered {
                answer,
                origin: how.origin,
            },
        )
        .await?;
    let result = crate::approval_run::apply_answer(runner, ctx, card, answer, how).await;
    refresh_last_ctx(runner, ctx);
    result.map(|_| ())
}

fn refresh_last_ctx(runner: &TurnRunner, ctx: &TurnCtx) {
    if let Ok(mut guard) = runner.last_ctx.lock()
        && guard.as_ref().is_some_and(|prev| prev.task == ctx.task)
    {
        *guard = Some(ctx.clone());
    }
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
            turn_started: 0,
            initiator: TurnInitiator::Verification,
            evidence: tokio::sync::Mutex::new(next),
            active_model: std::sync::Mutex::new(None),
            tools: Vec::new(),
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
