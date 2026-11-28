use std::sync::Arc;

use anybot_foundation::{CallId, DeviceId, TaskId};
use anybot_messages::Lang;
use anybot_protocol::{
    KnownTaskEventKind, QuestionAnswer, QuestionItem, TaskEvent, TaskEventKind, ToolEffect,
    ToolRequest, TurnInitiator,
};
use anybot_tools::{AskPersonArgs, CODE_QUESTION_SKIPPED, CODE_QUESTION_UNANSWERED};
use anybot_trust::OutcomeText;

use crate::error::TurnError;
use crate::runner::{KindCount, TurnCtx, TurnOutcome, TurnRunner, UserMessage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingQuestion {
    pub task: TaskId,
    pub card: String,
    pub call_id: CallId,
    pub device: DeviceId,
    pub questions: Vec<QuestionItem>,
}

pub(crate) async fn park_question(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    args: AskPersonArgs,
) -> Result<(), TurnError> {
    let args_summary = crate::action::args_json(&request.arguments)?;
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    let device = last_device(&events, &*runner.clock);
    let card = request.call_id.as_str();
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolIntent {
                call_id: request.call_id,
                name: request.tool.clone(),
                args_summary,
                level: ToolEffect::ReadOnly,
            },
        )
        .await?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolStarted {
                call_id: request.call_id,
            },
        )
        .await?;
    runner
        .append_task_event(
            &ctx.task,
            KnownTaskEventKind::QuestionShown {
                card: card.clone(),
                questions: args.questions.clone(),
            },
        )
        .await?;
    let Ok(mut guard) = runner.pending_question.lock() else {
        return Err(TurnError::ParkedBusy);
    };
    *guard = Some(PendingQuestion {
        task: ctx.task,
        card,
        call_id: request.call_id,
        device,
        questions: args.questions,
    });
    Ok(())
}

pub(crate) fn has_parked(runner: &TurnRunner) -> bool {
    let Ok(guard) = runner.pending_question.lock() else {
        return false;
    };
    guard.is_some()
}

pub(crate) fn take_parked(runner: &TurnRunner) -> Option<PendingQuestion> {
    let Ok(mut guard) = runner.pending_question.lock() else {
        return None;
    };
    guard.take()
}

fn take_matching(
    runner: &TurnRunner,
    task: TaskId,
    card: &str,
) -> Result<PendingQuestion, TurnError> {
    let Ok(mut guard) = runner.pending_question.lock() else {
        return Err(TurnError::ParkedBusy);
    };
    let matches = guard
        .as_ref()
        .is_some_and(|pending| pending.task == task && pending.card == card);
    if !matches {
        return Err(TurnError::QuestionNotFound {
            card: card.to_owned(),
        });
    }
    guard.take().ok_or_else(|| TurnError::QuestionNotFound {
        card: card.to_owned(),
    })
}

fn answer_summary(lang: Lang, answer: &QuestionAnswer) -> Result<String, TurnError> {
    match answer {
        QuestionAnswer::Answers(values) => Ok(values.join("\u{3001}")),
        QuestionAnswer::Skipped => {
            crate::view::render_text(lang, &OutcomeText::of(CODE_QUESTION_SKIPPED, Vec::new()))
        }
        QuestionAnswer::Unanswered => {
            crate::view::render_text(lang, &OutcomeText::of(CODE_QUESTION_UNANSWERED, Vec::new()))
        }
    }
}

fn last_device(events: &[TaskEvent], clock: &dyn anybot_foundation::Clock) -> DeviceId {
    for event in events.iter().rev() {
        if let TaskEventKind::Known(KnownTaskEventKind::TurnStarted {
            initiator: TurnInitiator::You { device },
            ..
        }) = &event.kind
        {
            return *device;
        }
    }
    DeviceId::generate(clock)
}

impl TurnRunner {
    pub async fn answer_question(
        &self,
        ctx: &TurnCtx,
        card: &str,
        answer: QuestionAnswer,
    ) -> Result<TurnOutcome, TurnError> {
        let Ok(guard) = Arc::clone(&self.active).try_lock_owned() else {
            return Err(TurnError::Busy);
        };
        let pending = take_matching(self, ctx.task, card)?;
        let result = self.finish_question(ctx, pending, answer).await;
        drop(guard);
        result
    }

    async fn finish_question(
        &self,
        ctx: &TurnCtx,
        pending: PendingQuestion,
        answer: QuestionAnswer,
    ) -> Result<TurnOutcome, TurnError> {
        if let Some(prev) = self.last_ctx().filter(|prev| prev.task == ctx.task) {
            ctx.advance_evidence_to(prev.peek_evidence());
        }
        let summary = answer_summary(self.lang, &answer)?;
        self.append_task_event(
            &ctx.task,
            KnownTaskEventKind::QuestionAnswered {
                card: pending.card,
                answer,
            },
        )
        .await?;
        self.append_task_event(
            &ctx.task,
            KnownTaskEventKind::ToolCompleted {
                call_id: pending.call_id,
                summary,
                raw_id: None,
            },
        )
        .await?;
        let turn = self
            .count_task_kind(&ctx.task, KindCount::TurnStarted)
            .await?
            .saturating_add(1);
        self.append_task_event(&ctx.task, self.turn_started_kind(turn, pending.device))
            .await?;
        let outcome = self.run_steps(ctx, ctx.task).await?;
        if let Ok(mut last) = self.last_ctx.lock() {
            *last = Some(ctx.clone());
        }
        Ok(outcome)
    }

    pub(crate) async fn continue_with_unanswered(
        &self,
        pending: PendingQuestion,
        message: UserMessage,
    ) -> Result<TurnOutcome, TurnError> {
        let summary = answer_summary(self.lang, &QuestionAnswer::Unanswered)?;
        self.append_task_event(
            &pending.task,
            KnownTaskEventKind::QuestionAnswered {
                card: pending.card,
                answer: QuestionAnswer::Unanswered,
            },
        )
        .await?;
        self.append_task_event(
            &pending.task,
            KnownTaskEventKind::ToolCompleted {
                call_id: pending.call_id,
                summary,
                raw_id: None,
            },
        )
        .await?;
        self.append_user_said(&pending.task, &message).await?;
        let turn = self
            .count_task_kind(&pending.task, KindCount::TurnStarted)
            .await?
            .saturating_add(1);
        self.append_task_event(&pending.task, self.turn_started_kind(turn, message.device))
            .await?;
        let ctx = self.approval_context(pending.task, turn);
        let outcome = self.run_steps(&ctx, pending.task).await?;
        if let Ok(mut last) = self.last_ctx.lock() {
            *last = Some(ctx.clone());
        }
        Ok(outcome)
    }
}
