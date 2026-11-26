use serde_json::{Value, json};

use crate::types::{
    Message, ProviderKind, Role, StopKind, ToolArguments, ToolCall, TurnRequest, TurnResult,
    UsageReport,
};
use crate::wire_openai_calls::PartialCall;
use crate::wire_openai_usage::usage_of;

pub(crate) const CHAT_PATH: &str = "/chat/completions";

#[must_use]
pub(crate) fn encode(
    request: &TurnRequest,
    model: &str,
    stream: bool,
    include_usage: bool,
) -> Value {
    let mut body = json!({
        "model": model,
        "messages": messages(request),
        "stream": stream,
    });
    if !request.tools.is_empty() {
        crate::jsonutil::insert(&mut body, "tools", json!(tools(&request.tools)));
    }
    if let Some(max) = request.max_output_tokens {
        crate::jsonutil::insert(&mut body, "max_tokens", json!(max));
    }
    if stream && include_usage {
        crate::jsonutil::insert(
            &mut body,
            "stream_options",
            json!({ "include_usage": true }),
        );
    }
    body
}

fn messages(request: &TurnRequest) -> Vec<Value> {
    let mut out = Vec::new();
    if !request.system.is_empty() {
        out.push(json!({ "role": "system", "content": request.system }));
    }
    for message in &request.messages {
        out.push(one_message(message));
    }
    out
}

fn one_message(message: &Message) -> Value {
    match message.role {
        Role::User => json!({ "role": "user", "content": message.text }),
        Role::Tool => json!({
            "role": "tool",
            "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
            "content": message.text,
        }),
        Role::Assistant => assistant_message(message),
    }
}

fn assistant_message(message: &Message) -> Value {
    let calls: Vec<Value> = message
        .tool_calls
        .iter()
        .map(|call| {
            json!({
                "id": call.id,
                "type": "function",
                "function": { "name": call.name, "arguments": arguments_text(&call.arguments) }
            })
        })
        .collect();
    let mut value = json!({ "role": "assistant", "content": message.text });
    if !calls.is_empty() {
        crate::jsonutil::insert(&mut value, "tool_calls", json!(calls));
    }
    value
}

fn arguments_text(arguments: &ToolArguments) -> String {
    match arguments {
        ToolArguments::Json(value) => value.to_string(),
        ToolArguments::Broken(raw) => raw.clone(),
    }
}

fn tools(specs: &[crate::types::ToolSpec]) -> Vec<Value> {
    specs
        .iter()
        .map(|spec| {
            json!({
                "type": "function",
                "function": {
                    "name": spec.name,
                    "description": spec.description,
                    "parameters": spec.parameters,
                }
            })
        })
        .collect()
}

#[must_use]
pub(crate) fn decode(body: &str, provider: ProviderKind, model: &str) -> TurnResult {
    let value = serde_json::from_str::<Value>(body).unwrap_or(Value::Null);
    decode_value(&value, provider, model, true)
}

#[must_use]
pub(crate) fn decode_stream(
    chunks: &[String],
    provider: ProviderKind,
    model: &str,
    closed: bool,
) -> TurnResult {
    let mut text = String::new();
    let mut thinking = String::new();
    let mut calls: Vec<PartialCall> = Vec::new();
    let mut finish = String::new();
    let mut usage = UsageReport::Absent;
    for chunk in chunks {
        let Some(value) = crate::wire_openai_text::parse_chunk(chunk) else {
            continue;
        };
        crate::wire_openai_text::apply_openai_chunk(
            &value,
            &mut text,
            &mut thinking,
            &mut calls,
            &mut finish,
            &mut usage,
        );
    }
    finish_result(
        assemble(calls, &finish, usage, text, thinking, closed),
        provider,
        model,
    )
}

fn decode_value(value: &Value, provider: ProviderKind, model: &str, closed: bool) -> TurnResult {
    let message = value
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .cloned()
        .unwrap_or(Value::Null);
    let finish = value
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("finish_reason"))
        .and_then(|r| r.as_str())
        .unwrap_or("stop");
    let text = message
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_owned();
    let mut thinking = String::new();
    crate::wire_openai_text::append_text(&mut thinking, message.get("reasoning"));
    crate::wire_openai_text::append_text(&mut thinking, message.get("reasoning_content"));
    let mut calls = Vec::new();
    if let Some(list) = message.get("tool_calls").and_then(|c| c.as_array()) {
        for (index, call) in list.iter().enumerate() {
            calls.push(call_from_object(call, index));
        }
    }
    let usage = usage_of(value).map_or(UsageReport::Absent, UsageReport::Present);
    let parts = Assembled {
        text,
        thinking,
        calls,
        finish: finish.to_owned(),
        usage,
        closed,
    };
    finish_result(parts, provider, model)
}

struct Assembled {
    text: String,
    thinking: String,
    calls: Vec<ToolCall>,
    finish: String,
    usage: UsageReport,
    closed: bool,
}

fn assemble(
    partials: Vec<PartialCall>,
    finish: &str,
    usage: UsageReport,
    text: String,
    thinking: String,
    closed: bool,
) -> Assembled {
    Assembled {
        text,
        thinking,
        calls: partials.into_iter().map(PartialCall::finish).collect(),
        finish: finish.to_owned(),
        usage,
        closed,
    }
}

fn finish_result(parts: Assembled, provider: ProviderKind, model: &str) -> TurnResult {
    let Assembled {
        text,
        thinking,
        calls,
        finish,
        usage,
        closed,
    } = parts;
    let json_calls = calls
        .iter()
        .filter(|call| matches!(call.arguments, ToolArguments::Json(_)) && !call.name.is_empty())
        .count();
    let stop = if !closed {
        StopKind::Interrupted
    } else if finish == "length" {
        StopKind::Length
    } else if json_calls > 0 {
        StopKind::NeedsTool
    } else {
        StopKind::Finished
    };
    let tool_calls = if matches!(stop, StopKind::Interrupted | StopKind::Length) {
        Vec::new()
    } else {
        calls.into_iter().filter(call_complete).collect()
    };
    TurnResult {
        text,
        thinking,
        tool_calls,
        usage,
        stop,
        attachment: None,
        model: model.to_owned(),
        provider,
    }
}

fn call_complete(call: &ToolCall) -> bool {
    !call.name.is_empty()
        && matches!(
            call.arguments,
            ToolArguments::Json(_) | ToolArguments::Broken(_)
        )
}

fn call_from_object(call: &Value, index: usize) -> ToolCall {
    let function = call.get("function").cloned().unwrap_or(Value::Null);
    PartialCall {
        index,
        id: call
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
        name: function
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
        arguments: function
            .get("arguments")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
    }
    .finish()
}
