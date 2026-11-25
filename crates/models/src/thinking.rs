use serde_json::{Value, json};

use crate::types::{ProviderKind, ThinkingMode, WireFormat};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ThinkingParam {
    pub field: &'static str,
    pub value: &'static str,
}

#[must_use]
pub(crate) fn thinking_param(
    kind: ProviderKind,
    format: WireFormat,
    mode: ThinkingMode,
) -> Option<ThinkingParam> {
    if matches!(mode, ThinkingMode::Auto) {
        return None;
    }
    if matches!(format, WireFormat::Anthropic) || matches!(kind, ProviderKind::Anthropic) {
        return Some(ThinkingParam {
            field: "thinking.budget_tokens",
            value: claude_budget(mode),
        });
    }
    Some(ThinkingParam {
        field: "reasoning_effort",
        value: openai_effort(mode),
    })
}

fn openai_effort(mode: ThinkingMode) -> &'static str {
    match mode {
        ThinkingMode::Fast => "low",
        ThinkingMode::Auto | ThinkingMode::Balanced => "medium",
        ThinkingMode::Deep => "high",
    }
}

fn claude_budget(mode: ThinkingMode) -> &'static str {
    match mode {
        ThinkingMode::Auto => "0",
        ThinkingMode::Fast => "1024",
        ThinkingMode::Balanced => "4096",
        ThinkingMode::Deep => "16000",
    }
}

#[must_use]
pub(crate) fn apply_thinking(
    body: &mut Value,
    kind: ProviderKind,
    format: WireFormat,
    mode: ThinkingMode,
) -> bool {
    let Some(param) = thinking_param(kind, format, mode) else {
        return false;
    };
    if matches!(format, WireFormat::Anthropic) {
        let Ok(budget) = param.value.parse::<u64>() else {
            return false;
        };
        crate::jsonutil::insert(
            body,
            "thinking",
            json!({ "type": "enabled", "budget_tokens": budget }),
        );
        return true;
    }
    crate::jsonutil::insert(body, param.field, json!(param.value));
    true
}
