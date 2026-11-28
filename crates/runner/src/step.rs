use anybot_context::{Compiled, NewEvent};
use anybot_domain::{ContextCollapseReason, ContextCollapsed, collapsed_kind, memory_dropped_kind};
use anybot_models::{Audience, GatewayFail, ask_plan};
use anybot_protocol::{KnownTaskEventKind, TurnEndReason};

use crate::action::{ActionLoop, run_actions};
use crate::answer;
use crate::checks::{Proceed, run_checks};
use crate::context_input::build_input;
use crate::continue_turn::next_asked_step;
use crate::ending::end_reason;
use crate::error::TurnError;
use crate::halt::finish_halt;
use crate::record::Appended;
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) enum StepFlow {
    Continue,
    Ended(TurnEndReason),
}

pub(crate) async fn run_step(runner: &TurnRunner, ctx: &TurnCtx) -> Result<StepFlow, TurnError> {
    match run_checks(runner, ctx).await? {
        Proceed::Halted(halt) => {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(StepFlow::Ended(reason));
        }
        Proceed::Go => {}
    }
    let input = build_input(runner, ctx).await?;
    let compiled = anybot_context::compile(&input).map_err(TurnError::Compile)?;
    match compiled {
        Compiled::Insufficient(short) => {
            let reason = TurnEndReason::ContextBudget {
                deficit_tokens: Some(short.deficit_tokens),
            };
            runner.append_turn_end(ctx, reason.clone(), false).await?;
            Ok(StepFlow::Ended(reason))
        }
        Compiled::Sent {
            request,
            manifest,
            events,
        } => run_sent(runner, ctx, &request, &manifest, &events).await,
    }
}

async fn run_sent(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &anybot_context::CompiledRequest,
    manifest: &anybot_context::Manifest,
    events: &[NewEvent],
) -> Result<StepFlow, TurnError> {
    let bytes = serde_json::to_vec(manifest)?;
    let id = runner.raw.put(bytes).await.map_err(TurnError::Raw)?;
    let manifest_id = id.hex();
    record_new_events(runner, ctx, manifest_id, manifest, events).await?;
    let turn_request = request
        .to_turn_request(
            runner.tools.clone(),
            runner.model.thinking,
            Some(manifest.budget.reserved_output),
        )
        .map_err(TurnError::Compile)?;
    let task_rows = runner
        .record
        .task_events(ctx.task)
        .await
        .map_err(TurnError::Record)?;
    let asked = next_asked_step(&task_rows);
    if let Appended::Halted(halt) = runner
        .append_turn_unless_halted(ctx, KnownTaskEventKind::ModelAsked { step: asked })
        .await?
    {
        let reason = finish_halt(runner, ctx, halt).await?;
        return Ok(StepFlow::Ended(reason));
    }
    let outcome = ask(runner, &turn_request).await;
    match outcome {
        Err(fail) => {
            if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
                let reason = finish_halt(runner, ctx, halt).await?;
                return Ok(StepFlow::Ended(reason));
            }
            let reason = end_reason(&fail);
            runner.append_turn_end(ctx, reason.clone(), false).await?;
            Ok(StepFlow::Ended(reason))
        }
        Ok(outcome) => finish_answer(runner, ctx, &outcome, manifest_id).await,
    }
}

async fn ask(
    runner: &TurnRunner,
    request: &anybot_models::TurnRequest,
) -> Result<anybot_models::AskOutcome, GatewayFail> {
    let mut session = runner.session.lock().await;
    let mut plan = ask_plan(&runner.model.primary, &[], request, Audience::Watching);
    plan.bot = &runner.bot.name;
    plan.stream = true;
    session.ask(&mut plan).await
}

async fn record_new_events(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    manifest_id: &str,
    manifest: &anybot_context::Manifest,
    events: &[NewEvent],
) -> Result<(), TurnError> {
    for event in events {
        let kind = match event {
            NewEvent::Collapsed { ids, reason } => collapsed_kind(&ContextCollapsed {
                ids: ids.clone(),
                reason: collapse_reason(*reason),
            }),
            NewEvent::MemoryDropped { ids } => memory_dropped_kind(ids),
            NewEvent::ContextBuilt => context_built_kind(manifest_id, &runner.model, manifest),
        };
        append_raw(runner, ctx, kind).await?;
    }
    Ok(())
}

fn context_built_kind(
    manifest_id: &str,
    model: &crate::runner::TurnModel,
    manifest: &anybot_context::Manifest,
) -> anybot_protocol::TaskEventKind {
    anybot_protocol::TaskEventKind::Known(KnownTaskEventKind::ContextBuilt {
        manifest_id: manifest_id.to_owned(),
        model: answer::model_name_of(&model.primary.endpoint),
        estimate_tokens: manifest.budget.estimate_calibrated,
    })
}

async fn append_raw(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    kind: anybot_protocol::TaskEventKind,
) -> Result<(), TurnError> {
    let new = anybot_record_new(runner, ctx, kind);
    runner
        .record
        .append(crate::record::RecordEntry::Task(new))
        .await
        .map(|_| ())
        .map_err(TurnError::Record)
}

fn anybot_record_new(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    kind: anybot_protocol::TaskEventKind,
) -> crate::record::NewTaskEvent {
    crate::record::NewTaskEvent {
        bot: runner.bot.id,
        task: ctx.task,
        turn: Some(ctx.turn),
        event_id: anybot_foundation::EventId::generate(&*runner.clock),
        at_ms: runner.clock.now_ms(),
        kind,
    }
}

fn collapse_reason(reason: anybot_context::CollapseReason) -> ContextCollapseReason {
    match reason {
        anybot_context::CollapseReason::Accumulated => ContextCollapseReason::Accumulated,
        anybot_context::CollapseReason::Budget => ContextCollapseReason::Budget,
        anybot_context::CollapseReason::Compact => ContextCollapseReason::Compact,
        anybot_context::CollapseReason::ModelSwapped => ContextCollapseReason::ModelSwapped,
        anybot_context::CollapseReason::ReplaceAll => ContextCollapseReason::ReplaceAll,
    }
}

async fn finish_answer(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    outcome: &anybot_models::AskOutcome,
    manifest_id: &str,
) -> Result<StepFlow, TurnError> {
    for handoff in &outcome.handoffs {
        runner
            .append_turn_event(ctx, answer::swapped_kind(handoff))
            .await?;
    }
    let native = keep_attachment(runner, outcome).await?;
    let result = &outcome.result;
    let requests = answer::requests_of(&result.tool_calls, &*runner.clock);
    let answer_kind = answer::answer_kind(result, &requests, native);
    runner.append_turn_event(ctx, answer_kind).await?;
    runner
        .append_turn_event(ctx, answer::usage_report_kind(&result.usage, manifest_id))
        .await?;
    if requests.is_empty() {
        runner
            .append_turn_end(ctx, TurnEndReason::Answered, true)
            .await?;
        return Ok(StepFlow::Ended(TurnEndReason::Answered));
    }
    if let ActionLoop::Halted(halt) = run_actions(runner, ctx, &requests).await? {
        let reason = finish_halt(runner, ctx, halt).await?;
        return Ok(StepFlow::Ended(reason));
    }
    if crate::approvals::has_parked(runner) {
        runner
            .append_turn_end(ctx, TurnEndReason::PendingApproval, false)
            .await?;
        return Ok(StepFlow::Ended(TurnEndReason::PendingApproval));
    }
    Ok(StepFlow::Continue)
}

async fn keep_attachment(
    runner: &TurnRunner,
    outcome: &anybot_models::AskOutcome,
) -> Result<Option<anybot_protocol::NativeRef>, TurnError> {
    let Some(attachment) = &outcome.result.attachment else {
        return Ok(None);
    };
    let id = runner
        .raw
        .put(attachment.bytes.clone())
        .await
        .map_err(TurnError::Raw)?;
    Ok(Some(answer::native_ref_of(attachment, id.hex())))
}
