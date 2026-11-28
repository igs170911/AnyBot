use anybot_foundation::{CallId, DeviceId, TaskId};
use anybot_messages::Lang;
use anybot_protocol::{
    KnownTaskEventKind, QuestionAnswer, QuestionItem, TaskEvent, TaskEventKind, ToolEffect,
    ToolRequest, TurnInitiator,
};
use anybot_tools::{AskPersonArgs, CODE_QUESTION_SKIPPED, CODE_QUESTION_UNANSWERED};
use anybot_trust::OutcomeText;

use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnOutcome, TurnRunner, UserMessage};

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
        let guard = self.lock_active()?;
        let _clear = crate::steer::ClearOnDrop(self);
        let pending = take_matching(self, ctx.task, card)?;
        let result = self.finish_question(pending, answer).await;
        drop(guard);
        result
    }

    async fn finish_question(
        &self,
        pending: PendingQuestion,
        answer: QuestionAnswer,
    ) -> Result<TurnOutcome, TurnError> {
        let initiator = TurnInitiator::You {
            device: pending.device,
        };
        let task = pending.task;
        self.close_question(pending, answer).await?;
        self.run_after_question(task, initiator).await
    }

    pub(crate) async fn continue_with_unanswered(
        &self,
        pending: PendingQuestion,
        message: UserMessage,
    ) -> Result<TurnOutcome, TurnError> {
        let task = pending.task;
        self.close_question(pending, QuestionAnswer::Unanswered)
            .await?;
        self.append_user_said(&task, &message).await?;
        let initiator = TurnInitiator::You {
            device: message.device,
        };
        self.run_after_question(task, initiator).await
    }

    async fn close_question(
        &self,
        pending: PendingQuestion,
        answer: QuestionAnswer,
    ) -> Result<(), TurnError> {
        let summary = answer_summary(self.lang, &answer)?;
        self.append_task_event(
            &pending.task,
            KnownTaskEventKind::QuestionAnswered {
                card: pending.card,
                answer,
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
        Ok(())
    }

    async fn run_after_question(
        &self,
        task: TaskId,
        initiator: TurnInitiator,
    ) -> Result<TurnOutcome, TurnError> {
        let ctx = crate::continue_turn::open_continued_ctx(self, task, initiator).await?;
        let reason = self.run_turn_loop(&ctx).await?;
        crate::steer::finish_or_continue(self, ctx, reason).await
    }
}
