use std::sync::Arc;

use anybot_protocol::{NotRunReason, ToolRequest};
use anybot_trust::{GateStep, OutcomeText};

use crate::action_record::{record_args_rejected, record_denied, record_executed, record_not_run};
use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};
use crate::view::ActionView;

pub(crate) async fn run_actions(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    requests: &[ToolRequest],
) -> Result<(), TurnError> {
    let mut refused = false;
    for request in requests {
        if refused {
            record_not_run(runner, ctx, request, NotRunReason::PreviousRefused).await?;
            continue;
        }
        refused = run_one(runner, ctx, request).await?;
    }
    Ok(())
}

async fn run_one(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    request: &ToolRequest,
) -> Result<bool, TurnError> {
    let json = args_json(&request.arguments)?;
    let offered: Vec<&str> = ctx.tools.iter().map(|spec| spec.name.as_str()).collect();
    if let Err(rejection) = anybot_tools::check_offered(&request.tool, &json, &offered) {
        record_args_rejected(runner, ctx, request, &rejection).await?;
        return Ok(false);
    }
    let args = match anybot_tools::parse_call(&request.tool, &json) {
        Ok(args) => args,
        Err(rejection) => {
            record_args_rejected(runner, ctx, request, &rejection).await?;
            return Ok(false);
        }
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
            Ok(true)
        }
        GateStep::Ask(ticket) => {
            let run = anybot_trust::ask_declined_run(&areq, &ticket, &view, &*runner.clock);
            record_denied(runner, ctx, request, &run).await?;
            crate::approvals::record_ask_summary(runner, ctx, request, &run, &view).await?;
            let parked_view = ActionView::new(runner.lang).with_computer(ticket.computer_name());
            crate::approvals::park_approval(runner, ctx, request, parked_view, *ticket).await?;
            Ok(true)
        }
        GateStep::Allow(ticket) => {
            record_executed(runner, ctx, request, &areq, ticket, &view, &*computer).await?;
            Ok(false)
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
