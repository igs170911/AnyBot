use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::output::{BlockKind, Compiled};
use anybot_models::{Role, ThinkingMode, ToolArguments, ToolSpec};

#[test]
fn turn_request_carries_plan_fields() {
    let mut base = sup::base_input();
    base.user_turns = vec![sup::record("u1", "hello there")];
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { request, .. } = compiled {
        let tools: Vec<ToolSpec> = vec![ToolSpec {
            name: "read_result".to_owned(),
            description: "read a record".to_owned(),
            parameters: serde_json::Value::Null,
        }];
        let turn = request.to_turn_request(tools.clone(), ThinkingMode::Auto, Some(1_024));
        assert_eq!(turn.system, request.system);
        assert_eq!(
            turn.messages.len(),
            request
                .blocks
                .iter()
                .filter(|b| b.kind != BlockKind::System)
                .count(),
        );
        assert_eq!(turn.tools, tools);
        assert_eq!(turn.max_output_tokens, Some(1_024));
        assert_eq!(turn.thinking, ThinkingMode::Auto);
        assert!(turn.cache_prefix);
        let user = request.blocks.iter().any(|b| b.kind == BlockKind::UserText);
        assert!(user);
        assert!(!turn.messages.is_empty());
    } else {
        panic!("expected sent");
    }
}

const CALL_READ: &str = "00000000-0000-0000-0000-000000000000";
const CALL_GREP: &str = "00000000-0000-0000-0000-000000000001";

fn sample_turn() -> anybot_models::TurnRequest {
    let mut base = sup::base_input();
    let mut read = sup::step_len(0, 40, "r0");
    read.tool = "read_file".to_owned();
    read.arguments = serde_json::json!({ "path": "src/lib.rs" });
    let mut grep = sup::step_len(1, 40, "r1");
    grep.tool = "grep".to_owned();
    grep.arguments = serde_json::json!({ "pattern": "tool" });
    base.steps = vec![read, grep];
    match compile(&base).unwrap() {
        Compiled::Sent { request, .. } => {
            request.to_turn_request(Vec::new(), ThinkingMode::Auto, None)
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

fn at<'a>(paired: &[&'a anybot_models::Message], index: usize) -> &'a anybot_models::Message {
    paired.get(index).expect("paired message")
}

fn call_of(message: &anybot_models::Message) -> &anybot_models::ToolCall {
    message.tool_calls.first().expect("call")
}

fn assert_intent(
    message: &anybot_models::Message,
    id: &str,
    name: &str,
    arguments: serde_json::Value,
) {
    assert_eq!(message.role, Role::Assistant);
    assert_eq!(call_of(message).id, id);
    assert_eq!(call_of(message).name, name);
    assert_eq!(call_of(message).arguments, ToolArguments::Json(arguments));
}

fn assert_result(message: &anybot_models::Message, id: &str) {
    assert_eq!(message.role, Role::Tool);
    assert_eq!(message.tool_call_id.as_deref(), Some(id));
    assert!(message.tool_calls.is_empty());
}

#[test]
fn tool_intent_stays_on_assistant_and_result_matches_call() {
    let turn = sample_turn();
    let paired: Vec<&anybot_models::Message> = turn
        .messages
        .iter()
        .filter(|m| !m.tool_calls.is_empty() || m.tool_call_id.is_some())
        .collect();
    assert_eq!(paired.len(), 4);
    assert_intent(
        at(&paired, 0),
        CALL_READ,
        "read_file",
        serde_json::json!({ "path": "src/lib.rs" }),
    );
    assert_result(at(&paired, 1), CALL_READ);
    assert_intent(
        at(&paired, 2),
        CALL_GREP,
        "grep",
        serde_json::json!({ "pattern": "tool" }),
    );
    assert_result(at(&paired, 3), CALL_GREP);
}

#[test]
fn intent_and_result_share_one_call_id() {
    let mut base = sup::base_input();
    base.steps = vec![sup::step_len(0, 400, "r0")];
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { request, .. } = compiled {
        let turn = request.to_turn_request(Vec::new(), ThinkingMode::Auto, None);
        let intent = turn
            .messages
            .iter()
            .find(|m| !m.tool_calls.is_empty())
            .expect("intent message");
        let result = turn
            .messages
            .iter()
            .find(|m| m.tool_call_id.is_some())
            .expect("result message");
        assert_eq!(
            intent.tool_calls.first().expect("call").id,
            result.tool_call_id.clone().expect("id")
        );
    } else {
        panic!("expected sent");
    }
}

#[test]
fn system_prompt_appears_once() {
    let mut base = sup::base_input();
    base.system_prompt = "ONLY-SYSTEM-TEXT".to_owned();
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { request, .. } = compiled {
        let turn = request.to_turn_request(Vec::new(), ThinkingMode::Auto, None);
        assert_eq!(turn.system, "ONLY-SYSTEM-TEXT");
        let hits = turn
            .messages
            .iter()
            .filter(|m| m.text.contains("ONLY-SYSTEM-TEXT"))
            .count();
        assert_eq!(hits, 0, "system text must not repeat inside messages");
    } else {
        panic!("expected sent");
    }
}

#[test]
fn manifest_estimator_names_the_formula_used() {
    let base = sup::base_input();
    let compiled = compile(&base).unwrap();
    if let Compiled::Sent { manifest, .. } = compiled {
        assert!(manifest.model.estimator.contains("text_tokens"));
    } else {
        panic!("expected sent");
    }
}
