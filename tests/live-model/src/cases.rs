use anybot_models::{ThinkingMode, ToolSpec, TurnRequest};

use test_harness::{Step, model_calls_tool, model_replies_text, model_think_then_reply};

use crate::probe::{
    TOOL_NAME, TOOL_SYSTEM, normal_request, stream_request, tool_arguments, tool_request,
    tool_schema,
};

pub const SCENARIOS: [&str; 3] = ["normal", "stream", "tool-call"];
pub const PRODUCT_SCENARIOS: [&str; 4] = ["normal", "stream", "tool-call", "thinking"];

pub struct ProbeCase {
    pub name: &'static str,
    pub body: String,
    pub stream: bool,
    pub step: Step,
}

#[must_use]
pub fn probe_cases(model: &str) -> Vec<ProbeCase> {
    vec![
        ProbeCase {
            name: SCENARIOS[0],
            body: normal_request(model),
            stream: false,
            step: probe_reply(false),
        },
        ProbeCase {
            name: SCENARIOS[1],
            body: stream_request(model),
            stream: true,
            step: probe_reply(true),
        },
        ProbeCase {
            name: SCENARIOS[2],
            body: tool_request(model),
            stream: false,
            step: model_calls_tool(TOOL_NAME, tool_arguments(), false),
        },
    ]
}

pub struct ProductCase {
    pub name: &'static str,
    pub request: TurnRequest,
    pub stream: bool,
    pub step: Step,
}

#[must_use]
pub fn product_cases() -> Vec<ProductCase> {
    product_cases_for(false)
}

#[must_use]
pub fn product_cases_for(always_thinks: bool) -> Vec<ProductCase> {
    vec![
        ProductCase {
            name: PRODUCT_SCENARIOS[0],
            request: turn_request(ThinkingMode::Auto),
            stream: false,
            step: reply_for(false, always_thinks),
        },
        ProductCase {
            name: PRODUCT_SCENARIOS[1],
            request: turn_request(ThinkingMode::Auto),
            stream: true,
            step: reply_for(true, always_thinks),
        },
        ProductCase {
            name: PRODUCT_SCENARIOS[2],
            request: turn_tool_request(),
            stream: false,
            step: model_calls_tool(TOOL_NAME, tool_arguments(), false),
        },
        ProductCase {
            name: PRODUCT_SCENARIOS[3],
            request: turn_request(ThinkingMode::Deep),
            stream: true,
            step: model_think_then_reply("plan", "ok", true),
        },
    ]
}

#[must_use]
pub fn fake_script(model: &str) -> Vec<Step> {
    script_for(model, false)
}

#[must_use]
pub fn script_for(model: &str, always_thinks: bool) -> Vec<Step> {
    let mut steps: Vec<Step> = probe_steps(model);
    steps.extend(product_steps_for(always_thinks));
    steps
}

#[must_use]
pub fn probe_steps(model: &str) -> Vec<Step> {
    probe_cases(model)
        .into_iter()
        .map(|case| case.step)
        .collect()
}

#[must_use]
pub fn product_steps() -> Vec<Step> {
    product_steps_for(false)
}

#[must_use]
pub fn product_steps_for(always_thinks: bool) -> Vec<Step> {
    product_cases_for(always_thinks)
        .into_iter()
        .map(|case| case.step)
        .collect()
}

pub const PROBE_REPLY_WORD: &str = "probe";

fn probe_reply(stream: bool) -> Step {
    model_think_then_reply("plan", PROBE_REPLY_WORD, stream)
}

fn product_reply(stream: bool) -> Step {
    model_replies_text("ok", stream)
}

fn reply_for(stream: bool, always_thinks: bool) -> Step {
    if always_thinks {
        return model_think_then_reply("plan", "ok", stream);
    }
    product_reply(stream)
}

fn turn_request(thinking: ThinkingMode) -> TurnRequest {
    TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: anybot_models::Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}

fn turn_tool_request() -> TurnRequest {
    let mut request = turn_request(ThinkingMode::Auto);
    TOOL_SYSTEM.clone_into(&mut request.system);
    request.tools.push(tool_spec());
    request
}

fn tool_spec() -> ToolSpec {
    let function = tool_schema()
        .get("function")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    ToolSpec {
        name: TOOL_NAME.to_owned(),
        description: function
            .get("description")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        parameters: function
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    }
}
