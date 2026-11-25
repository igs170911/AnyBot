use serde_json::json;
use tokio::sync::mpsc;

use crate::model_extra::openai_usage;
use crate::model_openai::FAKE_CREATED;
use crate::model_plan::json_line;
use crate::model_script::UsageScript;
use crate::model_vllm::VLLM_TOOL_FINISH;
use crate::model_vllm_body as body;

pub(crate) async fn think_tools(
    tx: &mpsc::Sender<String>,
    model: &str,
    thinking: &str,
    calls: &[crate::model_script::ToolCallScript],
) {
    send(tx, role_chunk(model)).await;
    send(tx, delta_chunk(model, &json!({ "reasoning": thinking }))).await;
    for (index, call) in calls.iter().enumerate() {
        send(
            tx,
            delta_chunk(
                model,
                &json!({ "tool_calls": [{ "index": index, "id": call.id, "type": "function",
                    "function": { "name": call.name, "arguments": call.arguments } }] }),
            ),
        )
        .await;
    }
    send(tx, finish_chunk(model, VLLM_TOOL_FINISH)).await;
}

pub(crate) async fn send(tx: &mpsc::Sender<String>, value: serde_json::Value) {
    let _sent = tx.send(json_line(&value)).await.is_ok();
}

pub(crate) fn usage_value_chunk(model: &str, usage: &UsageScript) -> serde_json::Value {
    let mut chunk = body::usage_chunk(model);
    if let Some(top) = chunk.as_object_mut() {
        top.insert("usage".to_owned(), openai_usage(usage));
    }
    chunk
}

pub(crate) fn content_chunk(model: &str, text: &str) -> serde_json::Value {
    delta_chunk(model, &json!({ "content": text }))
}

pub(crate) fn delta_chunk(model: &str, delta: &serde_json::Value) -> serde_json::Value {
    json!({
        "id": "chatcmpl-fake",
        "object": "chat.completion.chunk",
        "created": FAKE_CREATED,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "logprobs": null,
            "token_ids": null,
            "finish_reason": null,
        }],
        "kv_transfer_params": null,
    })
}

pub(crate) fn role_chunk(model: &str) -> serde_json::Value {
    let mut chunk = delta_chunk(model, &json!({ "role": "assistant", "content": "" }));
    let Some(top) = chunk.as_object_mut() else {
        return chunk;
    };
    top.insert("prompt_text".to_owned(), json!(null));
    top.insert("prompt_token_ids".to_owned(), json!(null));
    top.remove("kv_transfer_params");
    chunk
}

pub(crate) fn finish_chunk(model: &str, reason: &str) -> serde_json::Value {
    let mut chunk = delta_chunk(model, &json!({}));
    if let Some(choice) = chunk.get_mut("choices").and_then(|c| c.get_mut(0)) {
        choice["finish_reason"] = json!(reason);
    }
    chunk
}
