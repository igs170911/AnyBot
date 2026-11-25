use crate::types::NativeAttachment;
use crate::types::{ProviderKind, StopKind, ToolArguments, ToolCall, TurnResult, UsageReport};

pub(crate) struct ClaudeParts {
    pub text: String,
    pub thinking: String,
    pub calls: Vec<ToolCall>,
    pub stop: String,
    pub usage: UsageReport,
    pub attachment: Option<NativeAttachment>,
    pub closed: bool,
}

pub(crate) fn finish(parts: ClaudeParts, model: &str) -> TurnResult {
    let ClaudeParts {
        text,
        thinking,
        calls,
        stop,
        usage,
        attachment,
        closed,
    } = parts;
    let complete = calls
        .iter()
        .any(|call| matches!(call.arguments, ToolArguments::Json(_)));
    let kind = stop_kind(closed, &stop, complete, !calls.is_empty());
    let tool_calls = if matches!(kind, StopKind::Interrupted | StopKind::Length) {
        Vec::new()
    } else {
        calls
    };
    TurnResult {
        text,
        thinking,
        tool_calls,
        usage,
        stop: kind,
        attachment,
        model: model.to_owned(),
        provider: ProviderKind::Anthropic,
    }
}

fn stop_kind(closed: bool, stop: &str, complete: bool, has_calls: bool) -> StopKind {
    if !closed {
        return StopKind::Interrupted;
    }
    if stop == "max_tokens" {
        return StopKind::Length;
    }
    if stop == "refusal" {
        return StopKind::Refusal;
    }
    if complete || stop == "tool_use" && has_calls {
        return StopKind::NeedsTool;
    }
    StopKind::Finished
}
