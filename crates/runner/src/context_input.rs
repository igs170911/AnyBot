use anybot_context::{
    BuildInput, ChatRecord, CollapseReason, K114_2_DEFAULT_RESULT_CHARS, Manifest, ModelPlan,
    NewEvent, PinnedSnapshot,
};
use anybot_domain::{ContextCollapseReason, ContextCollapsed, collapsed_kind, memory_dropped_kind};
use anybot_foundation::TaskId;
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};

use crate::answer;
use crate::error::TurnError;
use crate::record::{NewTaskEvent, RecordEntry, Stored, StoredBody};
use crate::runner::{TurnCtx, TurnModel, TurnRunner};

pub(crate) async fn build_input(
    runner: &TurnRunner,
    ctx: &TurnCtx,
) -> Result<BuildInput, TurnError> {
    let model = ctx.active_model(&runner.model);
    build_input_for(runner, ctx, &model).await
}

pub(crate) async fn build_input_for(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    model: &TurnModel,
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
    let prefix = crate::action_events::computer_label_prefix(runner);
    let mut steps = crate::context_denied::steps_of(&stored, &ctx.task, runner.lang, &prefix)?;
    let view_limit = effective_view_limit(&mut steps);
    Ok(BuildInput {
        view_limit,
        events,
        user_turns: user_turns_of(&stored, &ctx.task),
        bot_turns: bot_turns_of(&stored, &ctx.task),
        steps,
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
        model: model_plan_of(model),
        compact_requested: false,
    })
}

const K114_2_SPECIAL_MAX_CHARS: usize = 21_000;

fn effective_view_limit(steps: &mut [anybot_context::StepRecord]) -> usize {
    let max_special = steps
        .iter()
        .filter(|s| s.tool == "shell" || s.tool == "read_result")
        .map(|s| s.content.chars().count())
        .max()
        .unwrap_or(0);
    if max_special <= K114_2_DEFAULT_RESULT_CHARS {
        return K114_2_DEFAULT_RESULT_CHARS;
    }
    for step in steps.iter_mut() {
        if step.tool != "shell"
            && step.tool != "read_result"
            && step.content.chars().count() > K114_2_DEFAULT_RESULT_CHARS
        {
            step.content = anybot_context::render_view(step, K114_2_DEFAULT_RESULT_CHARS);
        }
    }
    max_special.min(K114_2_SPECIAL_MAX_CHARS)
}

pub(crate) fn model_plan_of(model: &TurnModel) -> ModelPlan {
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

pub(crate) async fn record_new_events(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    manifest_id: &str,
    model: &TurnModel,
    manifest: &Manifest,
    events: &[NewEvent],
) -> Result<(), TurnError> {
    for event in events {
        let kind = match event {
            NewEvent::Collapsed { ids, reason } => collapsed_kind(&ContextCollapsed {
                ids: ids.clone(),
                reason: collapse_reason(*reason),
            }),
            NewEvent::MemoryDropped { ids } => memory_dropped_kind(ids),
            NewEvent::ContextBuilt => context_built_kind(manifest_id, model, manifest),
        };
        let new = NewTaskEvent {
            bot: runner.bot.id,
            task: ctx.task,
            turn: Some(ctx.turn),
            event_id: anybot_foundation::EventId::generate(&*runner.clock),
            at_ms: runner.clock.now_ms(),
            kind,
        };
        runner
            .record
            .append(RecordEntry::Task(new))
            .await
            .map(|_| ())
            .map_err(TurnError::Record)?;
    }
    Ok(())
}

fn context_built_kind(manifest_id: &str, model: &TurnModel, manifest: &Manifest) -> TaskEventKind {
    TaskEventKind::Known(KnownTaskEventKind::ContextBuilt {
        manifest_id: manifest_id.to_owned(),
        model: answer::model_name_of(&model.primary.endpoint),
        estimate_tokens: manifest.budget.estimate_calibrated,
    })
}

fn collapse_reason(reason: CollapseReason) -> ContextCollapseReason {
    match reason {
        CollapseReason::Accumulated => ContextCollapseReason::Accumulated,
        CollapseReason::Budget => ContextCollapseReason::Budget,
        CollapseReason::Compact => ContextCollapseReason::Compact,
        CollapseReason::ModelSwapped => ContextCollapseReason::ModelSwapped,
        CollapseReason::ReplaceAll => ContextCollapseReason::ReplaceAll,
    }
}
