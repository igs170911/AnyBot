use serde_json::{Value, json};

use crate::types::{NativeAttachment, ProviderKind, ToolArguments, ToolCall};

pub(crate) struct PartialTool {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl PartialTool {
    pub(crate) fn finish(self) -> ToolCall {
        let arguments = match serde_json::from_str::<Value>(&self.arguments) {
            Ok(value) => ToolArguments::Json(value),
            Err(_err) => ToolArguments::Broken(self.arguments),
        };
        ToolCall {
            id: if self.id.is_empty() {
                "toolu_0".to_owned()
            } else {
                self.id
            },
            name: self.name,
            arguments,
        }
    }
}

pub(crate) fn start_block(value: &Value, calls: &mut Vec<PartialTool>, redacted: &mut String) {
    let Some(block) = value.get("content_block") else {
        return;
    };
    match block.get("type").and_then(|t| t.as_str()).unwrap_or("") {
        "tool_use" => calls.push(PartialTool {
            index: usize::try_from(value.get("index").and_then(Value::as_u64).unwrap_or(0))
                .unwrap_or(0),
            id: block
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            name: block
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            arguments: String::new(),
        }),
        "redacted_thinking" => {
            crate::jsonutil::set_owned(
                redacted,
                block.get("data").and_then(Value::as_str).unwrap_or(""),
            );
        }
        _ => {}
    }
}

pub(crate) fn delta_block(
    value: &Value,
    text: &mut String,
    thinking: &mut String,
    signature: &mut String,
    calls: &mut [PartialTool],
) {
    let Some(delta) = value.get("delta") else {
        return;
    };
    match delta.get("type").and_then(|t| t.as_str()).unwrap_or("") {
        "text_delta" => text.push_str(delta.get("text").and_then(|v| v.as_str()).unwrap_or("")),
        "thinking_delta" => {
            thinking.push_str(delta.get("thinking").and_then(|v| v.as_str()).unwrap_or(""));
        }
        "signature_delta" => signature.push_str(
            delta
                .get("signature")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        "input_json_delta" => append_json(value, delta, calls),
        _ => {}
    }
}

fn append_json(value: &Value, delta: &Value, calls: &mut [PartialTool]) {
    let index =
        usize::try_from(value.get("index").and_then(Value::as_u64).unwrap_or(0)).unwrap_or(0);
    let piece = delta
        .get("partial_json")
        .and_then(Value::as_str)
        .unwrap_or("");
    if let Some(call) = calls.iter_mut().find(|call| call.index == index) {
        call.arguments.push_str(piece);
    } else if let Some(call) = calls.last_mut() {
        call.arguments.push_str(piece);
    }
}

pub(crate) fn tool_from_block(block: &Value, index: usize) -> ToolCall {
    let raw = block.get("input").cloned().unwrap_or(Value::Null);
    let arguments = if raw.is_object() {
        ToolArguments::Json(raw)
    } else {
        ToolArguments::Broken(raw.to_string())
    };
    let id = match block.get("id").and_then(Value::as_str) {
        Some(text) if !text.is_empty() => text.to_owned(),
        _ => format!("toolu_{index}"),
    };
    ToolCall {
        id,
        name: block
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
        arguments,
    }
}

#[must_use]
pub(crate) fn native_block(
    thinking: &str,
    signature: &str,
    redacted: &str,
    model: &str,
) -> Option<NativeAttachment> {
    let value = if !redacted.is_empty() {
        json!({ "type": "redacted_thinking", "data": redacted })
    } else if !signature.is_empty() {
        json!({ "type": "thinking", "thinking": thinking, "signature": signature })
    } else {
        return None;
    };
    Some(NativeAttachment {
        provider: ProviderKind::Anthropic,
        model: model.to_owned(),
        bytes: value.to_string().into_bytes(),
    })
}
