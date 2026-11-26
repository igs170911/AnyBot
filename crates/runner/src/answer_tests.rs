use serde_json::json;

use anybot_foundation::SystemClock;
use anybot_models::{
    HandoffRecord, ProviderKind, StopKind, SwitchReason, ToolArguments, ToolCall, TurnResult,
    UsageReport, record,
};
use anybot_protocol::{FinishReason, KnownTaskEventKind, ToolArgs};

use crate::answer::{answer_kind, requests_of, swapped_kind, thinking_of};

fn empty_result() -> TurnResult {
    TurnResult {
        text: String::new(),
        thinking: String::new(),
        tool_calls: Vec::new(),
        usage: UsageReport::Absent,
        stop: StopKind::Finished,
        attachment: None,
        model: String::new(),
        provider: ProviderKind::OpenAi,
    }
}

#[test]
fn answer_keeps_broken_arguments_and_empty_thinking_is_none() {
    assert_eq!(thinking_of(""), None);
    assert_eq!(thinking_of("plan"), Some("plan".to_owned()));
    let calls = vec![ToolCall {
        id: "c1".to_owned(),
        name: "read_file".to_owned(),
        arguments: ToolArguments::Broken("{\"path\":".to_owned()),
    }];
    let requests = requests_of(&calls, &SystemClock);
    assert_eq!(
        requests.first().map(|r| &r.arguments),
        Some(&ToolArgs::Broken {
            raw: "{\"path\":".to_owned()
        })
    );
    for (stop, finish) in [
        (StopKind::Finished, FinishReason::Finished),
        (StopKind::NeedsTool, FinishReason::NeedsTool),
        (StopKind::Length, FinishReason::Length),
        (StopKind::Refusal, FinishReason::Refusal),
        (StopKind::Interrupted, FinishReason::Interrupted),
    ] {
        let result = TurnResult {
            stop,
            ..empty_result()
        };
        let kind = answer_kind(&result, &[], None);
        let KnownTaskEventKind::ModelAnswer { finish: got, .. } = kind else {
            panic!("shape");
        };
        assert_eq!(got, finish);
    }
    let json_call = ToolCall {
        id: "c2".to_owned(),
        name: "shell".to_owned(),
        arguments: ToolArguments::Json(json!({ "command": "ls" })),
    };
    let kept = requests_of(&[json_call], &SystemClock);
    assert_eq!(
        kept.first().map(|r| &r.arguments),
        Some(&ToolArgs::Json {
            value: json!({ "command": "ls" })
        })
    );
}

#[test]
fn swapped_kind_from_handoff() {
    let handoff: HandoffRecord = record(
        ProviderKind::OpenAi,
        "a",
        ProviderKind::OpenAi,
        "b",
        SwitchReason::Fallback,
        "model-handoff-fallback",
        Vec::new(),
    );
    assert_eq!(
        swapped_kind(&handoff),
        KnownTaskEventKind::ModelSwapped {
            from: "a".to_owned(),
            to: "b".to_owned(),
            reason: "model-handoff-fallback".to_owned(),
        }
    );
}
