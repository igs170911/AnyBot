use std::sync::Arc;

use anybot_domain::task_view::BudgetOver;
use anybot_protocol::{NotRunReason, ToolRequest};
use anybot_trust::{GateStep, OutcomeText};

use crate::action_record::{record_args_rejected, record_denied, record_executed, record_not_run};
use crate::error::TurnError;
use crate::record::Halt;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) enum ActionLoop {
    Done,
    Halted(Halt),
    OverBudget(BudgetOver),
    TimedOut,
}

pub(crate) enum OneAction {
    Done,
    Refused,
    Halted(Halt),
    TimedOut,
}

struct LoopState {
    refused: bool,
    halted: Option<Halt>,
    over_budget: Option<BudgetOver>,
    timed_out: bool,
}

pub(crate) async fn run_actions(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    requests: &[ToolRequest],
) -> Result<ActionLoop, TurnError> {
    let mut state = LoopState {
        refused: false,
        halted: None,
        over_budget: None,
        timed_out: false,
    };
    for request in requests {
        step_request(runner, ctx, request, &mut state).await?;
    }
    if state.halted.is_none() && state.over_budget.is_none() && !state.timed_out {
        state.halted = crate::halt::check_halt(runner, ctx).await?;
    }
    if let Some(halt) = state.halted {
        return Ok(ActionLoop::Halted(halt));
    }
    if let Some(over) = state.over_budget {
        return Ok(ActionLoop::OverBudget(over));
    }
    if state.timed_out {
        return Ok(ActionLoop::TimedOut);
    }
    Ok(ActionLoop::Done)
}

async fn step_request(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    state: &mut LoopState,
) -> Result<(), TurnError> {
    if state.timed_out {
        return Ok(());
    }
    if state.halted.is_some() {
        return record_not_run(runner, ctx, request, NotRunReason::Stopped).await;
    }
    if state.over_budget.is_some() {
        return record_not_run(runner, ctx, request, NotRunReason::OverBudget).await;
    }
    if state.refused {
        return record_not_run(runner, ctx, request, NotRunReason::PreviousRefused).await;
    }
    if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
        state.halted = Some(halt);
        return record_not_run(runner, ctx, request, NotRunReason::Stopped).await;
    }
    if let Some(over) = crate::budget_gate::check_action_budget(runner, ctx).await? {
        state.over_budget = Some(over);
        return record_not_run(runner, ctx, request, NotRunReason::OverBudget).await;
    }
    match run_one(runner, ctx, request).await? {
        OneAction::Done => {}
        OneAction::Refused => state.refused = true,
        OneAction::Halted(halt) => state.halted = Some(halt),
        OneAction::TimedOut => state.timed_out = true,
    }
    Ok(())
}

async fn run_one(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
) -> Result<OneAction, TurnError> {
    let json = args_json(&request.arguments)?;
    let offered: Vec<&str> = ctx.tools.iter().map(|spec| spec.name.as_str()).collect();
    if let Err(rejection) = anybot_tools::check_offered(&request.tool, &json, &offered) {
        record_args_rejected(runner, ctx, request, &rejection).await?;
        return Ok(OneAction::Done);
    }
    let args = match anybot_tools::parse_call(&request.tool, &json) {
        Ok(args) => args,
        Err(rejection) => {
            record_args_rejected(runner, ctx, request, &rejection).await?;
            return Ok(OneAction::Done);
        }
    };
    match args {
        anybot_tools::ToolArgs::ReadResult(read) => {
            crate::read_result_run::run_read_result(runner, ctx, request, &read).await?;
            Ok(OneAction::Done)
        }
        anybot_tools::ToolArgs::AskPerson(ask_args) => {
            crate::questions::park_question(runner, ctx, request, ask_args).await?;
            Ok(OneAction::Refused)
        }
        other => run_gated(runner, ctx, request, other).await,
    }
}

async fn run_gated(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    args: anybot_tools::ToolArgs,
) -> Result<OneAction, TurnError> {
    let (areq, view) = build_areq(runner, ctx, request, args)?;
    if let Some(denial) = crate::reject_memory::repeat_denial(runner, request)? {
        let run = anybot_trust::step_denied_run(&areq, &denial, &*runner.clock);
        return apply_step(runner, ctx, request, &areq, view, GateStep::Denied(run)).await;
    }
    let call_ctx =
        anybot_trust::CallContext::new(request.call_id, ctx.turn_id, &request.provider_call_id);
    let step = crate::gate_run::decide(runner, &areq, &call_ctx, &view, true)?;
    apply_step(runner, ctx, request, &areq, view, step).await
}

pub(crate) fn build_areq(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    args: anybot_tools::ToolArgs,
) -> Result<(anybot_trust::ActionRequest, ActionView), TurnError> {
    let context = crate::action_record::action_context(runner, ctx, request)?;
    let view = view_of(runner.lang, &args).with_computer(&context.computer);
    Ok((
        anybot_tools::build_request(&request.tool, args, context),
        view,
    ))
}

pub(crate) async fn apply_step(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    areq: &anybot_trust::ActionRequest,
    view: ActionView,
    step: GateStep,
) -> Result<OneAction, TurnError> {
    match step {
        GateStep::Denied(run) => {
            record_denied(runner, ctx, request, &run).await?;
            crate::approvals::record_ask_summary(runner, ctx, request, &run, &view).await?;
            Ok(OneAction::Refused)
        }
        GateStep::Ask(ticket) => {
            crate::card_ask::handle_ask(runner, ctx, request, view, *ticket).await
        }
        GateStep::Allow(ticket) => {
            let computer = Arc::clone(&runner.gated);
            match record_executed(runner, ctx, request, areq, ticket, &view, &*computer).await? {
                Some(halt) => Ok(OneAction::Halted(halt)),
                None => Ok(OneAction::Done),
            }
        }
    }
}

pub(crate) fn args_json(args: &anybot_protocol::ToolArgs) -> Result<String, TurnError> {
    match args {
        anybot_protocol::ToolArgs::Json { value } => Ok(serde_json::to_string(value)?),
        anybot_protocol::ToolArgs::Broken { raw } => Ok(raw.clone()),
    }
}

fn view_of(lang: anybot_messages::Lang, args: &anybot_tools::ToolArgs) -> ActionView {
    match args {
        anybot_tools::ToolArgs::Edit(edit) => ActionView::new(lang)
            .with_edit_lines(line_count(&edit.new_text))
            .with_edit_path(&edit.path),
        anybot_tools::ToolArgs::Read(read) => ActionView::new(lang)
            .with_read_window(read.offset, read.limit)
            .with_read_path(&read.path),
        anybot_tools::ToolArgs::Search(search) => {
            ActionView::new(lang).with_search_files_only(search.files_only)
        }
        anybot_tools::ToolArgs::List(_)
        | anybot_tools::ToolArgs::ReadResult(_)
        | anybot_tools::ToolArgs::Shell(_)
        | anybot_tools::ToolArgs::AskPerson(_) => ActionView::new(lang),
    }
}

fn line_count(text: &str) -> u32 {
    let mut lines: u32 = 1;
    for ch in text.chars() {
        if ch == '\n' {
            lines = lines.saturating_add(1);
        }
    }
    if text.is_empty() {
        return 0;
    }
    lines
}

pub(crate) fn model_word(text: &OutcomeText) -> Option<String> {
    if text.code == anybot_trust::CODE_TURN_NOT_EXECUTED {
        return text
            .params
            .iter()
            .find(|(key, _)| key == "reason")
            .map(|(_, value)| value.clone());
    }
    None
}

pub(crate) fn not_run_reason_of(word: &str) -> NotRunReason {
    match word {
        "computer-unreachable" => NotRunReason::ComputerOffline,
        _ => NotRunReason::Stopped,
    }
}
