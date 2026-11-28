use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{AnswerOrigin, CardAnswer, ToolArgs, ToolRequest};

use super::RejectMemory;

fn deploy_request(line: &str) -> ToolRequest {
    ToolRequest {
        call_id: CallId::generate(&SystemClock),
        provider_call_id: "p".to_owned(),
        tool: "shell".to_owned(),
        arguments: ToolArgs::Json {
            value: serde_json::json!({ "line": line }),
        },
    }
}

fn remember_if_hand(memory: &RejectMemory, answer: CardAnswer, origin: AnswerOrigin) {
    if super::should_remember(answer, origin) {
        let request = deploy_request("make deploy");
        let args = crate::action::args_json(&request.arguments).unwrap();
        memory.remember(&request.tool, &args).unwrap();
    }
}

#[test]
fn hand_deny_is_remembered_by_tool_and_exact_arguments() {
    let memory = RejectMemory::default();
    remember_if_hand(&memory, CardAnswer::Deny, AnswerOrigin::Hand);
    let same = deploy_request("make deploy");
    let spaced = deploy_request("make  deploy");
    let same_args = crate::action::args_json(&same.arguments).unwrap();
    let spaced_args = crate::action::args_json(&spaced.arguments).unwrap();
    assert!(memory.contains("shell", &same_args).unwrap());
    assert!(!memory.contains("shell", &spaced_args).unwrap());
    assert!(!memory.contains("read_file", &same_args).unwrap());
}

#[test]
fn stop_auto_and_allow_answers_are_not_remembered() {
    let memory = RejectMemory::default();
    let args = crate::action::args_json(&deploy_request("make deploy").arguments).unwrap();
    for (answer, origin) in [
        (CardAnswer::Deny, AnswerOrigin::Stop),
        (CardAnswer::Deny, AnswerOrigin::Auto),
        (CardAnswer::AllowOnce, AnswerOrigin::Hand),
    ] {
        remember_if_hand(&memory, answer, origin);
        assert!(!memory.contains("shell", &args).unwrap());
    }
}

#[test]
fn clear_forgets_everything() {
    let memory = RejectMemory::default();
    remember_if_hand(&memory, CardAnswer::Deny, AnswerOrigin::Hand);
    memory.clear().unwrap();
    let args = crate::action::args_json(&deploy_request("make deploy").arguments).unwrap();
    assert!(!memory.contains("shell", &args).unwrap());
}
