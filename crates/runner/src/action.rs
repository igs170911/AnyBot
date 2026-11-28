use std::sync::Arc;

use anybot_domain::task_view::BudgetOver;
use anybot_protocol::{CardAnswer, CardKind, KnownTaskEventKind, NotRunReason, ToolRequest};
use anybot_trust::{AskTicket, GateStep, OutcomeText};

use crate::action_record::{record_args_rejected, record_denied, record_executed, record_not_run};
use crate::card_wait::CardWaitOutcome;
use crate::error::TurnError;
use crate::record::{Appended, Halt};
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
    let Ok(args) = anybot_tools::parse_call(&request.tool, &json) else {
        record_args_rejected(runner, ctx, request).await?;
        return Ok(OneAction::Done);
    };
    let context = crate::action_record::action_context(runner, ctx, request)?;
    let view = view_of(runner.lang, &args).with_computer(&context.computer);
    let areq = anybot_tools::build_request(&request.tool, args, context);
    let computer = Arc::clone(&runner.gated);
    let config = computer.gate_config();
    let call_ctx =
        anybot_trust::CallContext::new(request.call_id, ctx.turn_id, &request.provider_call_id);
    let step =
        anybot_trust::gate_step(&areq, &call_ctx, &*computer, &config, &view, &*runner.clock);
    match step {
        GateStep::Denied(run) => {
            record_denied(runner, ctx, request, &run).await?;
            crate::approvals::record_ask_summary(runner, ctx, request, &run, &view).await?;
            Ok(OneAction::Refused)
        }
        GateStep::Ask(ticket) => handle_ask(runner, ctx, request, view, *ticket).await,
        GateStep::Allow(ticket) => {
            match record_executed(runner, ctx, request, &areq, ticket, &view, &*computer).await? {
                Some(halt) => Ok(OneAction::Halted(halt)),
                None => Ok(OneAction::Done),
            }
        }
    }
}

async fn handle_ask(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
    view: ActionView,
    ticket: AskTicket,
) -> Result<OneAction, TurnError> {
    let shown = KnownTaskEventKind::CardShown {
        action: crate::approvals::first_action(&ticket),
        card: ticket.card_digest(),
        kind: CardKind::Action,
        resume: None,
    };
    let stored = match runner.append_turn_unless_halted(ctx, shown).await? {
        Appended::Halted(halt) => {
            record_not_run(runner, ctx, request, NotRunReason::Stopped).await?;
            return Ok(OneAction::Halted(halt));
        }
        Appended::Stored(row) => row,
    };
    let shown_seq = crate::card_wait::task_seq_of(&stored);
    match crate::card_wait::wait_for_card(runner, ctx, shown_seq).await? {
        CardWaitOutcome::Answered(answer) => {
            crate::approvals::finish_in_turn_approval(runner, ctx, request, &ticket, &view, answer)
                .await
        }
        CardWaitOutcome::Halted(halt) => {
            if halt == Halt::Stopped {
                runner
                    .append_turn_event(
                        ctx,
                        KnownTaskEventKind::CardAnswered {
                            answer: CardAnswer::Deny,
                        },
                    )
                    .await?;
            }
            record_not_run(runner, ctx, request, NotRunReason::Stopped).await?;
            Ok(OneAction::Halted(halt))
        }
        CardWaitOutcome::TimedOut => {
            crate::approvals::store_parked(runner, request, view, ticket)?;
            Ok(OneAction::TimedOut)
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
        anybot_tools::ToolArgs::Read(read) => {
            let from = read.offset.saturating_add(1);
            let to = from.saturating_add(read.limit.saturating_sub(1));
            ActionView::new(lang).with_read_range(u32_of(from), u32_of(to))
        }
        anybot_tools::ToolArgs::Shell(_) => ActionView::new(lang),
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

fn u32_of(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
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
