use std::sync::Arc;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::output::Compiled;
use anybot_foundation::clock::SystemClock;
use anybot_models::{ProviderKind, ThinkingMode, WireFormat, gateway, resolve_endpoint};
use test_harness::{ModelFormat, model_replies_text, model_start_with_format};

const CALL_READ: &str = "00000000-0000-0000-0000-000000000000";
const CALL_GREP: &str = "00000000-0000-0000-0000-000000000001";
const ARGS_READ: &str = r#"{"path":"src/lib.rs"}"#;
const ARGS_GREP: &str = r#"{"pattern":"tool"}"#;

fn sample_turn() -> anybot_models::TurnRequest {
    let mut base = sup::base_input();
    base.events = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "SHIP-MARK".to_owned(),
        },
    )];
    let mut read = sup::step_len(0, 40, "r0");
    read.tool = "read_file".to_owned();
    read.arguments = serde_json::json!({ "path": "src/lib.rs" });
    let mut grep = sup::step_len(1, 40, "r1");
    grep.tool = "grep".to_owned();
    grep.arguments = serde_json::json!({ "pattern": "tool" });
    base.steps = vec![read, grep];
    match compile(&base).unwrap() {
        Compiled::Sent { request, .. } => request
            .to_turn_request(Vec::new(), ThinkingMode::Auto, None)
            .unwrap(),
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[tokio::test]
async fn gateway_openai_and_claude_keep_tool_pairs() {
    let turn = sample_turn();
    let openai = recorded_body(ModelFormat::OpenAi, ProviderKind::OpenAi, "gpt-5", &turn).await;
    let claude = recorded_body(
        ModelFormat::Anthropic,
        ProviderKind::Anthropic,
        "claude-opus-5-5",
        &turn,
    )
    .await;
    assert_openai_pairs(&openai);
    assert_claude_pairs(&claude);
    assert!(openai.contains("\"role\":\"user\""));
    assert!(openai.contains("turn-program-note"));
    assert!(openai.contains("SHIP-MARK"));
    assert!(claude.contains("\"role\":\"user\""));
    assert!(claude.contains("turn-program-note"));
    assert!(claude.contains("SHIP-MARK"));
    assert!(last_is_user_or_tool(&openai));
    assert!(last_is_user_or_tool(&claude));
}

fn last_is_user_or_tool(body: &str) -> bool {
    let value: serde_json::Value = serde_json::from_str(body).expect("wire json");
    matches!(
        value
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|m| m.last())
            .and_then(role_of),
        Some("user" | "tool")
    )
}

async fn recorded_body(
    format: ModelFormat,
    kind: ProviderKind,
    model: &str,
    turn: &anybot_models::TurnRequest,
) -> String {
    let (fake, port) = model_start_with_format(format).await.expect("fake model");
    fake.script_set(vec![model_replies_text("ok", false)]);
    let wire = if matches!(kind, ProviderKind::Anthropic) {
        Some(WireFormat::Anthropic)
    } else {
        Some(WireFormat::OpenAiCompatible)
    };
    let endpoint = resolve_endpoint(
        kind,
        &format!("http://127.0.0.1:{port}/v1"),
        "test-key",
        model,
        wire,
    );
    let clock: Arc<dyn anybot_foundation::clock::Clock> = Arc::new(SystemClock);
    gateway(clock)
        .complete(&endpoint, turn, false)
        .await
        .expect("gateway complete");
    fake.recorded()
        .first()
        .expect("recorded request")
        .body
        .clone()
}

fn nth<'a>(tools: &[&'a serde_json::Value], index: usize) -> &'a serde_json::Value {
    tools.get(index).copied().expect("tool message")
}

fn assert_openai_pairs(body: &str) {
    let value: serde_json::Value = serde_json::from_str(body).expect("openai json");
    let tools = toolish(value.get("messages"));
    assert_eq!(tools.len(), 4);
    assert_openai_intent(nth(&tools, 0), CALL_READ, "read_file", ARGS_READ);
    assert_openai_result(nth(&tools, 1), CALL_READ);
    assert_openai_intent(nth(&tools, 2), CALL_GREP, "grep", ARGS_GREP);
    assert_openai_result(nth(&tools, 3), CALL_GREP);
}

fn assert_openai_intent(message: &serde_json::Value, id: &str, name: &str, arguments: &str) {
    assert_eq!(role_of(message), Some("assistant"));
    let call = first_call(message);
    assert_eq!(call.get("id").and_then(|v| v.as_str()), Some(id));
    assert_eq!(
        call.pointer("/function/name").and_then(|v| v.as_str()),
        Some(name)
    );
    assert_eq!(
        call.pointer("/function/arguments").and_then(|v| v.as_str()),
        Some(arguments)
    );
}

fn assert_openai_result(message: &serde_json::Value, id: &str) {
    assert_eq!(role_of(message), Some("tool"));
    assert_eq!(
        message.get("tool_call_id").and_then(|v| v.as_str()),
        Some(id)
    );
}

fn toolish(messages: Option<&serde_json::Value>) -> Vec<&serde_json::Value> {
    messages
        .and_then(|m| m.as_array())
        .expect("messages")
        .iter()
        .filter(|m| {
            m.get("tool_calls").is_some() || m.get("role").and_then(|r| r.as_str()) == Some("tool")
        })
        .collect()
}

fn role_of(message: &serde_json::Value) -> Option<&str> {
    message.get("role").and_then(|r| r.as_str())
}

fn first_call(message: &serde_json::Value) -> &serde_json::Value {
    message
        .get("tool_calls")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .expect("call")
}

fn assert_claude_pairs(body: &str) {
    let value: serde_json::Value = serde_json::from_str(body).expect("claude json");
    let tools: Vec<&serde_json::Value> = value
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("messages")
        .iter()
        .filter(|m| message_has_tool(m))
        .collect();
    assert_eq!(tools.len(), 4);
    assert_claude_intent(
        nth(&tools, 0),
        CALL_READ,
        "read_file",
        &serde_json::json!({ "path": "src/lib.rs" }),
    );
    assert_claude_result(nth(&tools, 1), CALL_READ);
    assert_claude_intent(
        nth(&tools, 2),
        CALL_GREP,
        "grep",
        &serde_json::json!({ "pattern": "tool" }),
    );
    assert_claude_result(nth(&tools, 3), CALL_GREP);
}

fn assert_claude_intent(
    message: &serde_json::Value,
    id: &str,
    name: &str,
    input: &serde_json::Value,
) {
    assert_eq!(role_of(message), Some("assistant"));
    let block = content_of(message, "tool_use");
    assert_eq!(block.get("id").and_then(|v| v.as_str()), Some(id));
    assert_eq!(block.get("name").and_then(|v| v.as_str()), Some(name));
    assert_eq!(block.get("input"), Some(input));
}

fn assert_claude_result(message: &serde_json::Value, id: &str) {
    assert_eq!(role_of(message), Some("user"));
    assert_eq!(
        content_of(message, "tool_result")
            .get("tool_use_id")
            .and_then(|v| v.as_str()),
        Some(id)
    );
}

fn message_has_tool(message: &serde_json::Value) -> bool {
    message
        .get("content")
        .and_then(|c| c.as_array())
        .is_some_and(|blocks| {
            blocks.iter().any(|block| {
                matches!(
                    block.get("type").and_then(|t| t.as_str()),
                    Some("tool_use" | "tool_result")
                )
            })
        })
}

fn content_of<'a>(message: &'a serde_json::Value, kind: &str) -> &'a serde_json::Value {
    message
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|block| block.get("type").and_then(|t| t.as_str()) == Some(kind))
        })
        .expect("content block")
}
