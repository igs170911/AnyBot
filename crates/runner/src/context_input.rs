use anybot_context::{
    BuildInput, ChatRecord, K114_2_DEFAULT_RESULT_CHARS, ModelPlan, PinnedSnapshot,
};
use anybot_foundation::TaskId;
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};

use crate::error::TurnError;
use crate::record::{Stored, StoredBody};
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) async fn build_input(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<BuildInput, TurnError> {
    let events = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    let stored = runner
        .record
        .conversation(runner.bot.id)
        .await
        .map_err(TurnError::Record)?;
    let system_prompt =
        crate::fixed_blocks::system_prompt(&runner.bot, &runner.computer, runner.lang)?;
    Ok(BuildInput {
        view_limit: K114_2_DEFAULT_RESULT_CHARS,
        events,
        user_turns: user_turns_of(&stored, &ctx.task),
        bot_turns: bot_turns_of(&stored, &ctx.task),
        steps: crate::context_denied::steps_of(&stored, &ctx.task, runner.lang)?,
        constraints: Vec::new(),
        open_questions: Vec::new(),
        tool_specs: ctx.tools.clone(),
        system_prompt,
        pinned: PinnedSnapshot {
            version: String::new(),
            items: Vec::new(),
        },
        memories: Vec::new(),
        prior_chat: Vec::new(),
        model: model_plan_of(&runner.model),
        compact_requested: false,
    })
}

pub(crate) fn model_plan_of(model: &crate::runner::TurnModel) -> ModelPlan {
    ModelPlan {
        capability: model.capability.clone(),
        calibration: model.calibration.clone(),
        reserved_output: model.reserved_output,
    }
}

fn user_turns_of(stored: &[Stored], task: &TaskId) -> Vec<ChatRecord> {
    stored
        .iter()
        .filter_map(|row| match &row.body {
            StoredBody::UserSaid(line) if line.task == *task => Some(ChatRecord {
                record_id: line.record_id.as_str(),
                text: line.text.clone(),
                seq: row.stream_seq,
            }),
            StoredBody::UserSaid(_) | StoredBody::Task { .. } => None,
        })
        .collect()
}

pub(crate) fn known_of<'a>(row: &'a Stored, task: &TaskId) -> Option<&'a KnownTaskEventKind> {
    match &row.body {
        StoredBody::Task {
            task: owner, event, ..
        } if owner == task => match &event.kind {
            TaskEventKind::Known(kind) => Some(kind),
            TaskEventKind::Raw(_) => None,
        },
        StoredBody::UserSaid(_) | StoredBody::Task { .. } => None,
    }
}

fn bot_turns_of(stored: &[Stored], task: &TaskId) -> Vec<ChatRecord> {
    stored
        .iter()
        .filter_map(|row| match known_of(row, task) {
            Some(KnownTaskEventKind::ModelAnswer { text, .. }) if !text.is_empty() => {
                row_event_id(row).map(|id| ChatRecord {
                    record_id: id.as_str(),
                    text: text.clone(),
                    seq: row.stream_seq,
                })
            }
            Some(_) | None => None,
        })
        .collect()
}

pub(crate) fn row_event_id(row: &Stored) -> Option<anybot_foundation::EventId> {
    match &row.body {
        StoredBody::Task { event, .. } => Some(event.event_id),
        StoredBody::UserSaid(_) => None,
    }
}
