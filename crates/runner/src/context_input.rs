use std::collections::BTreeMap;

use anybot_context::{
    BuildInput, ChatRecord, K114_2_DEFAULT_RESULT_CHARS, ModelPlan, PinnedSnapshot,
    StepOutcomeKind, StepRecord,
};
use anybot_foundation::{CallId, TaskId};
use anybot_protocol::{KnownTaskEventKind, TaskEventKind, ToolArgs, effect_level};

use crate::error::TurnError;
use crate::fixed_blocks::not_wired_text;
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
        steps: steps_of(&stored, &ctx.task, runner.lang)?,
        constraints: Vec::new(),
        open_questions: Vec::new(),
        tool_specs: runner.tools.clone(),
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

fn row_event_id(row: &Stored) -> Option<anybot_foundation::EventId> {
    match &row.body {
        StoredBody::Task { event, .. } => Some(event.event_id),
        StoredBody::UserSaid(_) => None,
    }
}

fn not_runs_of(
    stored: &[Stored],
    task: &TaskId,
) -> BTreeMap<CallId, (u64, anybot_foundation::EventId)> {
    let mut map = BTreeMap::new();
    for row in stored {
        let Some(KnownTaskEventKind::ToolNotRun { call_id, .. }) = known_of(row, task) else {
            continue;
        };
        let Some(event_id) = row_event_id(row) else {
            continue;
        };
        map.insert(*call_id, (row.stream_seq, event_id));
    }
    map
}

fn answers_of(stored: &[Stored], task: &TaskId) -> Vec<Vec<ToolArgsPair>> {
    stored
        .iter()
        .filter_map(|row| match known_of(row, task) {
            Some(KnownTaskEventKind::ModelAnswer { tool_requests, .. }) => Some(
                tool_requests
                    .iter()
                    .map(|r| ToolArgsPair {
                        call_id: r.call_id,
                        tool: r.tool.clone(),
                        arguments: r.arguments.clone(),
                    })
                    .collect(),
            ),
            Some(_) | None => None,
        })
        .collect()
}

struct ToolArgsPair {
    call_id: CallId,
    tool: String,
    arguments: ToolArgs,
}

pub(crate) fn steps_of(
    stored: &[Stored],
    task: &TaskId,
    lang: anybot_messages::Lang,
) -> Result<Vec<StepRecord>, TurnError> {
    let not_runs = not_runs_of(stored, task);
    let mut steps: Vec<StepRecord> = Vec::new();
    for answer in answers_of(stored, task) {
        for request in answer {
            let Some((seq, event_id)) = not_runs.get(&request.call_id) else {
                return Err(TurnError::UnpairedToolRequest {
                    call_id: request.call_id,
                });
            };
            steps.push(step_record_of(lang, &request, *seq, event_id)?);
        }
    }
    Ok(steps)
}

fn step_record_of(
    lang: anybot_messages::Lang,
    request: &ToolArgsPair,
    seq: u64,
    event_id: &anybot_foundation::EventId,
) -> Result<StepRecord, TurnError> {
    let (summary, value) = args_text(&request.arguments)?;
    Ok(StepRecord {
        seq,
        call_id: request.call_id,
        tool: request.tool.clone(),
        args_summary: summary,
        arguments: value,
        level: effect_level(None),
        unconfirmed: false,
        outcome: StepOutcomeKind::Failed,
        source_ref: event_id.as_str(),
        content: not_wired_text(lang, &request.tool)?,
        exit_code: None,
        error_at: Vec::new(),
    })
}

fn args_text(args: &ToolArgs) -> Result<(String, serde_json::Value), TurnError> {
    match args {
        ToolArgs::Json { value } => Ok((serde_json::to_string(value)?, value.clone())),
        ToolArgs::Broken { raw } => Ok((raw.clone(), serde_json::Value::String(raw.clone()))),
    }
}
