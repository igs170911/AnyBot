use anybot_context::{Compiled, NewEvent};
use anybot_protocol::{BudgetItem, KnownTaskEventKind, TurnEndReason};

use crate::action::{ActionLoop, run_actions};
use crate::answer;
use crate::checks::{Proceed, run_checks};
use crate::context_input::{build_input, record_new_events};
use crate::continue_turn::next_asked_step;
use crate::error::TurnError;
use crate::fallback::{FallbackResult, ask_step_with, handle_ask_fail};
use crate::halt::finish_halt;
use crate::record::Appended;
use crate::runner::{TurnCtx, TurnModel, TurnRunner};

pub(crate) enum StepFlow {
    Continue,
    Ended(TurnEndReason),
}

pub(crate) enum PreparedSent {
    Ready {
        manifest_id: String,
        turn_request: anybot_models::TurnRequest,
    },
    Halted(TurnEndReason),
}

pub(crate) async fn run_step(runner: &TurnRunner, ctx: &TurnCtx) -> Result<StepFlow, TurnError> {
    match run_checks(runner, ctx).await? {
        Proceed::Halted(halt) => {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(StepFlow::Ended(reason));
        }
        Proceed::OverBudget { item, used, limit } => {
            return end_over_budget(runner, ctx, item, used, limit).await;
        }
        Proceed::Go => {}
    }
    crate::steer::consume_for_step(runner, ctx).await?;
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
    let active = ctx.active_model(&runner.model);
    let (manifest_id, turn_request) =
        match prepare_sent(runner, ctx, &active, request, manifest, events).await? {
            PreparedSent::Ready {
                manifest_id,
                turn_request,
            } => (manifest_id, turn_request),
            PreparedSent::Halted(reason) => return Ok(StepFlow::Ended(reason)),
        };
    match ask_step_with(runner, &active, &turn_request).await {
        Ok(outcome) => finish_answer(runner, ctx, &outcome, &manifest_id).await,
        Err(fail) => match handle_ask_fail(runner, ctx, &active, fail).await? {
            FallbackResult::Answered {
                outcome,
                manifest_id,
            } => finish_answer(runner, ctx, &outcome, &manifest_id).await,
            FallbackResult::Ended(flow) => Ok(flow),
        },
    }
}

pub(crate) async fn prepare_sent(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    model: &TurnModel,
    request: &anybot_context::CompiledRequest,
    manifest: &anybot_context::Manifest,
    events: &[NewEvent],
) -> Result<PreparedSent, TurnError> {
    let bytes = serde_json::to_vec(manifest)?;
    let id = runner.raw.put(bytes).await.map_err(TurnError::Raw)?;
    let manifest_id = id.hex().to_owned();
    record_new_events(runner, ctx, &manifest_id, model, manifest, events).await?;
    let turn_request = request
        .to_turn_request(
            ctx.tools.clone(),
            model.thinking,
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
        return Ok(PreparedSent::Halted(reason));
    }
    Ok(PreparedSent::Ready {
        manifest_id,
        turn_request,
    })
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
    record_budget_usage(runner, ctx, &result.usage).await?;
    if requests.is_empty() {
        if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(StepFlow::Ended(reason));
        }
        if let Some(over) = crate::budget_gate::current_over(runner, ctx).await? {
            return end_over_budget(runner, ctx, over.item, over.used, over.limit).await;
        }
        crate::ending::maybe_record_claim_check(runner, ctx, &result.text, result.stop).await?;
        runner
            .append_turn_end(ctx, TurnEndReason::Answered, true)
            .await?;
        return Ok(StepFlow::Ended(TurnEndReason::Answered));
    }
    match run_actions(runner, ctx, &requests).await? {
        ActionLoop::Halted(halt) => {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(StepFlow::Ended(reason));
        }
        ActionLoop::OverBudget(over) => {
            return end_over_budget(runner, ctx, over.item, over.used, over.limit).await;
        }
        ActionLoop::TimedOut => {
            let reason = TurnEndReason::CardTimedOut;
            runner.append_turn_end(ctx, reason.clone(), false).await?;
            return Ok(StepFlow::Ended(reason));
        }
        ActionLoop::Done => {}
    }
    if crate::questions::has_parked(runner) {
        let reason = TurnEndReason::PendingApproval;
        runner.append_turn_end(ctx, reason.clone(), false).await?;
        return Ok(StepFlow::Ended(reason));
    }
    Ok(StepFlow::Continue)
}

async fn record_budget_usage(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    usage: &anybot_models::UsageReport,
) -> Result<(), TurnError> {
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::BudgetUsage {
                item: BudgetItem::Steps,
                delta: 1,
            },
        )
        .await?;
    if let Some(tokens) = crate::budget_gate::token_delta(usage) {
        runner
            .append_turn_event(
                ctx,
                KnownTaskEventKind::BudgetUsage {
                    item: BudgetItem::Tokens,
                    delta: tokens,
                },
            )
            .await?;
    }
    Ok(())
}

async fn end_over_budget(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    item: BudgetItem,
    used: u64,
    limit: u64,
) -> Result<StepFlow, TurnError> {
    let reason = TurnEndReason::OverBudget { item, used, limit };
    runner.append_turn_end(ctx, reason.clone(), false).await?;
    Ok(StepFlow::Ended(reason))
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
