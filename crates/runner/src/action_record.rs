use std::sync::Arc;

use anybot_protocol::{KnownTaskEventKind, NotRunReason, ToolRequest};
use anybot_trust::{ActionContext, ActionRequest, ActionRun, OpTicket};

use crate::action::not_run_reason_of;
use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) fn action_context(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
) -> Result<ActionContext, TurnError> {
    let computer = Arc::clone(&runner.gated);
    let classes = classes_of(request, runner.lang)?;
    Ok(ActionContext {
        task: ctx.task,
        turn: ctx.turn_id,
        evidence_no: ctx.peek_evidence(),
        computer: computer.name().to_owned(),
        workdir: computer.declared_workdir(),
        tool: request.tool.clone(),
        classes,
    })
}

fn classes_of(
    request: &ToolRequest,
    _lang: anybot_messages::Lang,
) -> Result<Vec<String>, TurnError> {
    let json = crate::action::args_json(&request.arguments)?;
    let Ok(anybot_tools::ToolArgs::Shell(shell)) = anybot_tools::parse_call(&request.tool, &json)
    else {
        return Ok(Vec::new());
    };
    let classes = anybot_trust::classes_of_command(&shell.line);
    Ok(classes.iter().map(ToString::to_string).collect())
}

pub(crate) async fn record_not_run(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    reason: NotRunReason,
) -> Result<(), TurnError> {
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ToolNotRun {
                call_id: request.call_id,
                reason,
            },
        )
        .await
        .map(|_| ())
}

pub(crate) async fn record_args_rejected(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
) -> Result<(), TurnError> {
    let json = crate::action::args_json(&request.arguments)?;
    let reason = anybot_tools::args_rejection_word(&request.tool, &json);
    runner
        .append_turn_event(
            ctx,
            KnownTaskEventKind::ToolFailed {
                call_id: request.call_id,
                name: request.tool.clone(),
                args_summary: json,
                error: reason,
            },
        )
        .await
        .map(|_| ())
}

pub(crate) async fn record_denied(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    run: &ActionRun,
) -> Result<(), TurnError> {
    record_gate(runner, ctx, request.call_id, &run.gate).await?;
    record_evidence(runner, ctx, &run.evidence).await?;
    Ok(())
}

pub(crate) async fn record_executed(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    areq: &ActionRequest,
    ticket: OpTicket,
    view: &ActionView,
    computer: &dyn anybot_trust::GatedComputer,
) -> Result<(), TurnError> {
    let run = anybot_trust::exec_step(areq, ticket, computer, view, &*runner.clock);
    record_gate(runner, ctx, request.call_id, &run.gate).await?;
    if let Some(word) = crate::action::model_word(&run.summary.status) {
        record_not_run(runner, ctx, request, not_run_reason_of(&word)).await?;
        return Ok(());
    }
    record_intent_start(runner, ctx, request, &run.gate).await?;
    record_result(runner, ctx, request, &run, view).await?;
    let ids = record_evidence(runner, ctx, &run.evidence).await?;
    record_verified(runner, ctx, request.call_id, &run, ids).await
}

pub(crate) use crate::action_events::{
    record_evidence, record_gate, record_intent_start, record_result, record_verified,
};
