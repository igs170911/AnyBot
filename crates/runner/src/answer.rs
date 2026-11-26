use anybot_foundation::CallId;
use anybot_models::{HandoffRecord, NativeAttachment, StopKind, ToolArguments, UsageReport};
use anybot_protocol::{
    FinishReason, KnownTaskEventKind, ModelName, NativeRef, TokenUsage, ToolArgs, ToolRequest,
};

#[must_use]
pub fn finish_of(stop: StopKind) -> FinishReason {
    match stop {
        StopKind::Finished => FinishReason::Finished,
        StopKind::NeedsTool => FinishReason::NeedsTool,
        StopKind::Length => FinishReason::Length,
        StopKind::Refusal => FinishReason::Refusal,
        StopKind::Interrupted => FinishReason::Interrupted,
    }
}

#[must_use]
pub fn args_of(arguments: &ToolArguments) -> ToolArgs {
    match arguments {
        ToolArguments::Json(value) => ToolArgs::Json {
            value: value.clone(),
        },
        ToolArguments::Broken(raw) => ToolArgs::Broken { raw: raw.clone() },
    }
}

#[must_use]
pub fn requests_of(
    calls: &[anybot_models::ToolCall],
    clock: &dyn anybot_foundation::Clock,
) -> Vec<ToolRequest> {
    calls
        .iter()
        .map(|call| ToolRequest {
            call_id: CallId::generate(clock),
            provider_call_id: call.id.clone(),
            tool: call.name.clone(),
            arguments: args_of(&call.arguments),
        })
        .collect()
}

#[must_use]
pub fn usage_of(usage: &UsageReport) -> Option<TokenUsage> {
    match usage {
        UsageReport::Absent => None,
        UsageReport::Present(values) => Some(TokenUsage {
            input: values.input_tokens,
            cache_read: values.cache_read_tokens,
            cache_write: values.cache_write_tokens,
            output: values.output_tokens,
            thinking: values.thinking_tokens,
        }),
    }
}

#[must_use]
pub fn thinking_of(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    Some(text.to_owned())
}

#[must_use]
pub fn native_ref_of(attachment: &NativeAttachment, blob_hex: &str) -> NativeRef {
    NativeRef {
        provider: format!("{:?}", attachment.provider),
        model: attachment.model.clone(),
        blob: blob_hex.to_owned(),
    }
}

#[must_use]
pub fn answer_kind(
    result: &anybot_models::TurnResult,
    requests: &[ToolRequest],
    native: Option<NativeRef>,
) -> KnownTaskEventKind {
    KnownTaskEventKind::ModelAnswer {
        text: result.text.clone(),
        thinking: thinking_of(&result.thinking),
        tool_requests: requests.to_vec(),
        finish: finish_of(result.stop),
        native,
    }
}

#[must_use]
pub fn usage_report_kind(usage: &UsageReport, manifest_id: &str) -> KnownTaskEventKind {
    KnownTaskEventKind::ModelUsage {
        usage: usage_of(usage),
        manifest_id: manifest_id.to_owned(),
    }
}

#[must_use]
pub fn swapped_kind(handoff: &HandoffRecord) -> KnownTaskEventKind {
    KnownTaskEventKind::ModelSwapped {
        from: handoff.before_model.clone(),
        to: handoff.after_model.clone(),
        reason: handoff.reason_code.to_owned(),
    }
}

#[must_use]
pub fn model_name_of(endpoint: &anybot_models::ProviderEndpoint) -> ModelName {
    ModelName {
        provider: format!("{:?}", endpoint.kind),
        base_url: endpoint.base_url.clone(),
        model: endpoint.model.clone(),
    }
}

#[must_use]
pub fn tool_not_run(call_id: &CallId) -> KnownTaskEventKind {
    KnownTaskEventKind::ToolNotRun {
        call_id: *call_id,
        reason: anybot_protocol::NotRunReason::NotWired,
    }
}

#[cfg(test)]
#[path = "answer_tests.rs"]
mod answer_tests;
