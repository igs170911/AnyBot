use serde_json::{Value, json};

use crate::providers::CLAUDE_MAX_OUTPUT;
use crate::types::{
    Message, ProviderKind, Role, ToolArguments, TurnRequest, TurnResult, Usage, UsageReport,
};
use crate::wire_claude_blocks::{
    PartialTool, delta_block, native_block, start_block, tool_from_block,
};
use crate::wire_claude_finish::{ClaudeParts, finish};
use crate::wire_claude_usage::{merge_usage, usage_of};

pub(crate) const MESSAGES_PATH: &str = "/messages";
pub(crate) const ANTHROPIC_VERSION: &str = "2023-06-01";

#[must_use]
pub(crate) fn encode(request: &TurnRequest, model: &str, stream: bool) -> Value {
    let max_tokens = request
        .max_output_tokens
        .unwrap_or(CLAUDE_MAX_OUTPUT)
        .min(CLAUDE_MAX_OUTPUT);
    let mut body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages(request, model),
        "stream": stream,
    });
    if !request.system.is_empty() {
        crate::jsonutil::insert(
            &mut body,
            "system",
            system_blocks(&request.system, request.cache_prefix),
        );
    }
    if !request.tools.is_empty() {
        crate::jsonutil::insert(
            &mut body,
            "tools",
            json!(tools(&request.tools, request.cache_prefix)),
        );
    }
    body
}

fn system_blocks(system: &str, cache: bool) -> Value {
    let mut block = json!({ "type": "text", "text": system });
    if cache {
        crate::jsonutil::insert(&mut block, "cache_control", json!({ "type": "ephemeral" }));
    }
    json!([block])
}

fn tools(specs: &[crate::types::ToolSpec], cache: bool) -> Vec<Value> {
    specs
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let mut tool = json!({
                "name": spec.name,
                "description": spec.description,
                "input_schema": spec.parameters,
            });
            if cache && index.saturating_add(1) == specs.len() {
                crate::jsonutil::insert(&mut tool, "cache_control", json!({ "type": "ephemeral" }));
            }
            tool
        })
        .collect()
}

fn messages(request: &TurnRequest, model: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for message in &request.messages {
        match message.role {
            Role::User => out.push(json!({ "role": "user", "content": message.text })),
            Role::Tool => out.push(tool_result(message)),
            Role::Assistant => out.push(assistant(message, request, model)),
        }
    }
    out
}

fn tool_result(message: &Message) -> Value {
    let id = claude_id(message.tool_call_id.as_deref().unwrap_or_default());
    json!({
        "role": "user",
        "content": [{
            "type": "tool_result",
            "tool_use_id": id,
            "content": message.text,
        }]
    })
}

fn assistant(message: &Message, request: &TurnRequest, model: &str) -> Value {
    let mut blocks = Vec::new();
    for attachment in &request.attachments {
        if attachment.provider == ProviderKind::Anthropic
            && attachment.model == model
            && let Ok(text) = std::str::from_utf8(&attachment.bytes)
            && let Ok(value) = serde_json::from_str::<Value>(text)
        {
            blocks.push(value);
        }
    }
    if !message.text.is_empty() {
        blocks.push(json!({ "type": "text", "text": message.text }));
    }
    for call in &message.tool_calls {
        blocks.push(json!({
            "type": "tool_use",
            "id": claude_id(&call.id),
            "name": call.name,
            "input": arguments_value(&call.arguments),
        }));
    }
    json!({ "role": "assistant", "content": blocks })
}

fn arguments_value(arguments: &ToolArguments) -> Value {
    match arguments {
        ToolArguments::Json(value) => value.clone(),
        ToolArguments::Broken(raw) => json!({ "raw": raw }),
    }
}

#[must_use]
pub(crate) fn claude_id(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("toolu_0");
    }
    out
}

#[must_use]
pub(crate) fn decode(body: &str, model: &str) -> TurnResult {
    let value = serde_json::from_str::<Value>(body).unwrap_or(Value::Null);
    decode_value(&value, model, true)
}

#[must_use]
pub(crate) fn decode_stream(events: &[String], model: &str, closed: bool) -> TurnResult {
    let mut text = String::new();
    let mut thinking = String::new();
    let mut signature = String::new();
    let mut redacted = String::new();
    let mut calls: Vec<PartialTool> = Vec::new();
    let mut stop = String::new();
    let mut usage = Usage::default_zero();
    let mut saw_usage = false;
    for event in events {
        let Ok(value) = serde_json::from_str::<Value>(event) else {
            continue;
        };
        match value.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "content_block_start" => start_block(&value, &mut calls, &mut redacted),
            "content_block_delta" => {
                delta_block(&value, &mut text, &mut thinking, &mut signature, &mut calls);
            }
            "message_delta" => {
                if let Some(reason) = value
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(Value::as_str)
                {
                    crate::jsonutil::set_owned(&mut stop, reason);
                }
                if let Some(parsed) = usage_of(value.get("usage")) {
                    merge_usage(&mut usage, &parsed);
                    saw_usage = true;
                }
            }
            "message_start" => {
                if let Some(parsed) = usage_of(value.get("message").and_then(|m| m.get("usage"))) {
                    merge_usage(&mut usage, &parsed);
                    saw_usage = true;
                }
            }
            _ => {}
        }
    }
    let report = if saw_usage {
        UsageReport::Present(usage)
    } else {
        UsageReport::Absent
    };
    let attachment = native_block(&thinking, &signature, &redacted, model);
    let finished = calls.into_iter().map(PartialTool::finish).collect();
    finish(
        ClaudeParts {
            text,
            thinking,
            calls: finished,
            stop,
            usage: report,
            attachment,
            closed,
        },
        model,
    )
}

fn decode_value(value: &Value, model: &str, closed: bool) -> TurnResult {
    let mut text = String::new();
    let mut thinking = String::new();
    let mut signature = String::new();
    let mut redacted = String::new();
    let mut calls = Vec::new();
    if let Some(blocks) = value.get("content").and_then(|c| c.as_array()) {
        for block in blocks {
            match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                "text" => text.push_str(block.get("text").and_then(|t| t.as_str()).unwrap_or("")),
                "thinking" => {
                    thinking.push_str(block.get("thinking").and_then(Value::as_str).unwrap_or(""));
                    crate::jsonutil::set_owned(
                        &mut signature,
                        block.get("signature").and_then(Value::as_str).unwrap_or(""),
                    );
                }
                "redacted_thinking" => {
                    crate::jsonutil::set_owned(
                        &mut redacted,
                        block.get("data").and_then(Value::as_str).unwrap_or(""),
                    );
                }
                "tool_use" => calls.push(tool_from_block(block, calls.len())),
                _ => {}
            }
        }
    }
    let stop = value
        .get("stop_reason")
        .and_then(|r| r.as_str())
        .unwrap_or("end_turn");
    let usage = usage_of(value.get("usage")).map_or(UsageReport::Absent, UsageReport::Present);
    let attachment = native_block(&thinking, &signature, &redacted, model);
    finish(
        ClaudeParts {
            text,
            thinking,
            calls,
            stop: stop.to_owned(),
            usage,
            attachment,
            closed,
        },
        model,
    )
}
