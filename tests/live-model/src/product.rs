use std::sync::Arc;

use anybot_foundation::clock::{Clock, SystemClock};
use anybot_models::{
    ProviderKind, StopKind, ThinkingMode, TurnRequest, TurnResult, gateway, resolve_endpoint,
};
use test_harness::{
    ModelFormat, model_calls_tool, model_replies_text, model_start_with_format,
    model_think_then_reply,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeutralShape {
    pub stop: String,
    pub has_text: bool,
    pub has_thinking: bool,
    pub tool_calls: usize,
    pub usage: String,
}

pub async fn compare_fakes() -> Result<(), String> {
    let left = shapes_from(ModelFormat::Vllm).await?;
    let right = shapes_from(ModelFormat::Vllm).await?;
    if left != right {
        return Err(format!("neutral shapes differ: {left:?} vs {right:?}"));
    }
    Ok(())
}

async fn shapes_from(format: ModelFormat) -> Result<Vec<NeutralShape>, String> {
    let (model, port) = model_start_with_format(format).await?;
    model.script_set(vec![
        model_replies_text("ok", false),
        model_replies_text("ok", true),
        model_calls_tool("read", serde_json::json!({}), false),
        model_think_then_reply("plan", "ok", true),
    ]);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let gateway = gateway(clock);
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        "local",
        None,
    );
    let request = hi();
    let mut shapes = Vec::new();
    for stream in [false, true, false, true] {
        let result = gateway
            .complete(&endpoint, &request, stream)
            .await
            .map_err(|err| format!("{err:?}"))?;
        shapes.push(shape_of(&result));
    }
    let _held = model;
    Ok(shapes)
}

fn hi() -> TurnRequest {
    TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: anybot_models::Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking: ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}

fn shape_of(result: &TurnResult) -> NeutralShape {
    NeutralShape {
        stop: match result.stop {
            StopKind::Finished => "finished",
            StopKind::NeedsTool => "needs-tool",
            StopKind::Length => "length",
            StopKind::Refusal => "refusal",
            StopKind::Interrupted => "interrupted",
        }
        .to_owned(),
        has_text: !result.text.is_empty(),
        has_thinking: !result.thinking.is_empty(),
        tool_calls: result.tool_calls.len(),
        usage: match result.usage {
            anybot_models::UsageReport::Absent => "absent",
            anybot_models::UsageReport::Present(_) => "present",
        }
        .to_owned(),
    }
}

#[must_use]
pub fn missing_profile_hint() -> &'static str {
    "create .local/live-model/profile.toml with base_url, model, ca_file, key_file"
}
