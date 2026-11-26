#[derive(Clone, Debug)]
pub(crate) struct ReplyCase {
    pub format: u8,
    pub text: String,
    pub thinking: String,
    pub tool: String,
    pub args: String,
    pub finish: String,
    pub extra_key: String,
    pub extra_value: String,
    pub stream: bool,
    pub cut: usize,
}

pub(crate) fn events_of(case: &ReplyCase) -> Vec<String> {
    match case.format {
        2 => claude_events(case),
        1 => openai_events(case, true),
        _ => openai_events(case, false),
    }
}

fn openai_events(case: &ReplyCase, vllm: bool) -> Vec<String> {
    let mut events = vec![chunk(&openai_start(case, vllm))];
    push_openai_parts(&mut events, case, vllm);
    events.push(chunk(&openai_end(case, vllm)));
    events
}

fn push_openai_parts(events: &mut Vec<String>, case: &ReplyCase, vllm: bool) {
    if !case.thinking.is_empty() {
        let field = if vllm {
            "reasoning"
        } else {
            "reasoning_content"
        };
        events.push(chunk(&serde_json::json!({
            "choices": [{ "delta": { field: case.thinking }, "finish_reason": null }]
        })));
    }
    if !case.text.is_empty() {
        events.push(chunk(&serde_json::json!({
            "choices": [{ "delta": { "content": case.text }, "finish_reason": null }]
        })));
    }
    if !case.tool.is_empty() {
        events.push(chunk(&tool_delta(case)));
    }
}

fn openai_start(case: &ReplyCase, vllm: bool) -> serde_json::Value {
    let mut value = serde_json::json!({
        "choices": [{ "delta": { "role": "assistant" }, "finish_reason": null }]
    });
    stamp(&mut value, case, vllm);
    value
}

fn openai_end(case: &ReplyCase, vllm: bool) -> serde_json::Value {
    let mut value = serde_json::json!({
        "choices": [{ "delta": {}, "finish_reason": openai_finish(case, vllm) }]
    });
    stamp(&mut value, case, vllm);
    value
}

fn stamp(value: &mut serde_json::Value, case: &ReplyCase, vllm: bool) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    if !case.extra_key.is_empty() {
        object.insert(case.extra_key.clone(), serde_json::json!(case.extra_value));
    }
    if vllm {
        object.insert("kv_transfer_params".to_owned(), serde_json::json!(null));
        object.insert(
            "routed_experts".to_owned(),
            serde_json::json!(case.extra_value),
        );
        object.insert("token_ids".to_owned(), serde_json::json!([1]));
        object.insert("metrics".to_owned(), serde_json::json!({"ms": 1}));
    }
}

fn tool_delta(case: &ReplyCase) -> serde_json::Value {
    serde_json::json!({
        "choices": [{
            "delta": { "tool_calls": [{
                "index": 0,
                "id": "call_1",
                "function": { "name": case.tool, "arguments": case.args }
            }] },
            "finish_reason": null
        }]
    })
}

pub(crate) fn openai_finish(case: &ReplyCase, vllm: bool) -> String {
    if vllm && !case.tool.is_empty() {
        return "stop".to_owned();
    }
    case.finish.clone()
}

fn claude_events(case: &ReplyCase) -> Vec<String> {
    let mut events = Vec::new();
    let index = push_claude_thinking(&mut events, case);
    push_claude_text(&mut events, case, index);
    push_claude_tool(&mut events, case, index);
    events.push(chunk(&claude_stop(case)));
    events.push(chunk(&serde_json::json!({ "type": "message_stop" })));
    events
}

fn push_claude_thinking(events: &mut Vec<String>, case: &ReplyCase) -> u64 {
    if case.thinking.is_empty() {
        return 0;
    }
    events.push(chunk(&claude_think_start(case, 0)));
    events.push(chunk(&serde_json::json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": { "type": "thinking_delta", "thinking": case.thinking }
    })));
    1
}

fn push_claude_tool(events: &mut Vec<String>, case: &ReplyCase, index: u64) {
    if case.tool.is_empty() {
        return;
    }
    let tool_index = index.saturating_add(u64::from(!case.text.is_empty()));
    events.push(chunk(&claude_tool_start(case, tool_index)));
    events.push(chunk(&serde_json::json!({
        "type": "content_block_delta",
        "index": tool_index,
        "delta": { "type": "input_json_delta", "partial_json": case.args }
    })));
}

fn push_claude_text(events: &mut Vec<String>, case: &ReplyCase, index: u64) {
    if case.text.is_empty() {
        return;
    }
    events.push(chunk(&serde_json::json!({
        "type": "content_block_start",
        "index": index,
        "content_block": { "type": "text", "text": "" }
    })));
    events.push(chunk(&serde_json::json!({
        "type": "content_block_delta",
        "index": index,
        "delta": { "type": "text_delta", "text": case.text }
    })));
}

fn claude_think_start(case: &ReplyCase, index: u64) -> serde_json::Value {
    let mut value = serde_json::json!({
        "type": "content_block_start",
        "index": index,
        "content_block": { "type": "thinking", "thinking": "", "signature": "sig" }
    });
    stamp_claude(&mut value, case);
    value
}

fn claude_tool_start(case: &ReplyCase, index: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "content_block_start",
        "index": index,
        "content_block": {
            "type": "tool_use",
            "id": "toolu_1",
            "name": case.tool,
            "input": {}
        }
    })
}

fn claude_stop(case: &ReplyCase) -> serde_json::Value {
    let mut value = serde_json::json!({
        "type": "message_delta",
        "delta": { "stop_reason": claude_finish(case) }
    });
    stamp_claude(&mut value, case);
    value
}

fn stamp_claude(value: &mut serde_json::Value, case: &ReplyCase) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    if !case.extra_key.is_empty() {
        object.insert(case.extra_key.clone(), serde_json::json!(case.extra_value));
    }
}

pub(crate) fn claude_finish(case: &ReplyCase) -> String {
    match case.finish.as_str() {
        "length" => "max_tokens".to_owned(),
        "refusal" => "refusal".to_owned(),
        "tool_calls" | "tool_use" => "tool_use".to_owned(),
        other => other.to_owned(),
    }
}

fn chunk(value: &serde_json::Value) -> String {
    value.to_string()
}

pub(crate) fn sse(events: &[String], done: bool) -> String {
    let mut raw = String::new();
    for event in events {
        raw.push_str("data: ");
        raw.push_str(event);
        raw.push_str("\n\n");
    }
    if done {
        raw.push_str("data: [DONE]\n\n");
    }
    raw
}

pub(crate) fn whole_body(case: &ReplyCase) -> String {
    match case.format {
        2 => claude_whole(case),
        _ => openai_whole(case),
    }
}

fn openai_whole(case: &ReplyCase) -> String {
    let mut value = serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": case.text,
                "reasoning": case.thinking,
                "tool_calls": tool_message(case),
            },
            "finish_reason": openai_finish(case, case.format == 1),
        }]
    });
    stamp(&mut value, case, case.format == 1);
    value.to_string()
}

fn tool_message(case: &ReplyCase) -> serde_json::Value {
    if case.tool.is_empty() {
        return serde_json::json!([]);
    }
    serde_json::json!([{
        "id": "call_1",
        "type": "function",
        "function": { "name": case.tool, "arguments": case.args }
    }])
}

fn claude_whole(case: &ReplyCase) -> String {
    let mut value = serde_json::json!({
        "content": claude_content(case),
        "stop_reason": claude_finish(case),
    });
    stamp_claude(&mut value, case);
    value.to_string()
}

fn claude_content(case: &ReplyCase) -> Vec<serde_json::Value> {
    let mut content = Vec::new();
    if !case.thinking.is_empty() {
        content.push(serde_json::json!({
            "type": "thinking",
            "thinking": case.thinking,
            "signature": "sig"
        }));
    }
    if !case.text.is_empty() {
        content.push(serde_json::json!({ "type": "text", "text": case.text }));
    }
    if !case.tool.is_empty() {
        content.push(serde_json::json!({
            "type": "tool_use",
            "id": "toolu_1",
            "name": case.tool,
            "input": serde_json::from_str::<serde_json::Value>(&case.args)
                .unwrap_or(serde_json::Value::Null)
        }));
    }
    content
}
