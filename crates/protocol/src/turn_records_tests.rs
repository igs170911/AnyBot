use super::{
    ComputerRef, FinishReason, ModelName, NotRunReason, TaskOpener, ThinkingChoice, TokenUsage,
    ToolArgs, ToolRequest, TurnEndReason, TurnInitiator,
};
use anybot_foundation::{CallId, DeviceId, SystemClock};

fn json_of<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn t_task_object_2() {
    assert_eq!(json_of(&TaskOpener::You), serde_json::json!("you"));
    let clock = SystemClock;
    let device = DeviceId::generate(&clock);
    let initiator = TurnInitiator::You { device };
    let got = json_of(&initiator);
    assert_eq!(got.get("kind").unwrap(), "you");
    assert_eq!(
        got.get("device").unwrap().as_str().unwrap(),
        device.as_str()
    );
}

#[test]
fn t_task_events_2() {
    let reason = TurnEndReason::ContextBudget {
        deficit_tokens: Some(7),
    };
    let got = json_of(&reason);
    assert_eq!(got.get("kind").unwrap(), "context_budget");
    assert_eq!(got.get("deficit_tokens").unwrap(), 7);
    assert_eq!(
        json_of(&TurnEndReason::Answered).get("kind").unwrap(),
        "answered"
    );
    let failed = TurnEndReason::ModelFailed {
        code: "model-auth".to_owned(),
    };
    assert_eq!(json_of(&failed).get("code").unwrap(), "model-auth");
}

#[test]
fn t_models_normalize_5() {
    let broken = ToolArgs::Broken {
        raw: "{\"path\":".to_owned(),
    };
    let got = json_of(&broken);
    assert_eq!(got.get("kind").unwrap(), "broken");
    assert_eq!(got.get("raw").unwrap(), "{\"path\":");
    let value = serde_json::json!({"path": "a.txt"});
    let json_args = ToolArgs::Json {
        value: value.clone(),
    };
    let got = json_of(&json_args);
    assert_eq!(got.get("kind").unwrap(), "json");
    assert_eq!(got.get("value").unwrap().clone(), value);
}

#[test]
fn tool_request_round_trips() {
    let clock = SystemClock;
    let request = ToolRequest {
        call_id: CallId::generate(&clock),
        provider_call_id: "c1".to_owned(),
        tool: "read_file".to_owned(),
        arguments: ToolArgs::Json {
            value: serde_json::json!({"path": "a.txt"}),
        },
    };
    let text = serde_json::to_string(&request).unwrap();
    let back: ToolRequest = serde_json::from_str(&text).unwrap();
    assert_eq!(back, request);
}

#[test]
fn finish_reasons_and_thinking_choices_serialize_snake_case() {
    assert_eq!(
        json_of(&FinishReason::NeedsTool),
        serde_json::json!("needs_tool")
    );
    assert_eq!(
        json_of(&FinishReason::Interrupted),
        serde_json::json!("interrupted")
    );
    assert_eq!(
        json_of(&ThinkingChoice::Balanced),
        serde_json::json!("balanced")
    );
    assert_eq!(
        json_of(&NotRunReason::NotWired),
        serde_json::json!("not_wired")
    );
}

#[test]
fn names_and_usage_serialize_with_named_fields() {
    let model = ModelName {
        provider: "OpenAi".to_owned(),
        base_url: "http://127.0.0.1:1/v1".to_owned(),
        model: "gpt-5".to_owned(),
    };
    let got = json_of(&model);
    assert_eq!(got.get("provider").unwrap(), "OpenAi");
    let computer = ComputerRef {
        name: "bot-computer".to_owned(),
        platform: super::ComputerPlatform::BotMachine,
    };
    let got = json_of(&computer);
    assert_eq!(got.get("platform").unwrap(), "bot_machine");
    let usage = TokenUsage {
        input: 12,
        cache_read: 0,
        cache_write: 0,
        output: 3,
        thinking: 0,
    };
    let got = json_of(&usage);
    assert_eq!(got.get("input").unwrap(), 12);
    assert_eq!(got.get("thinking").unwrap(), 0);
}

#[test]
fn delivery_report_claims_and_relayed_round_trip() {
    let report = super::DeliveryReportRecord {
        goal: "send file".to_owned(),
        status: super::TaskStatus::Completed,
        detail: Some(super::TaskDetail::Accepted),
        summary: super::ReportSummary::Text("done".to_owned()),
        attention: vec![super::ReportAttention {
            kind: super::AttentionKind::Rejected,
            tool: "shell".to_owned(),
            args_summary: "ls".to_owned(),
            error: "gate-denied".to_owned(),
        }],
        steps: Vec::new(),
        claims: vec![super::ReportClaim {
            sentence: "done".to_owned(),
            level: super::ReportLevel::Inferred,
            reasons: vec![super::ReasonPayload {
                code: "claim-no-evidence".to_owned(),
                params: Vec::new(),
            }],
            source: Some(super::JudgementSource::ForwardedUnverified),
            cited: vec![super::EvidenceShortId(1)],
        }],
        level_counts: super::LevelCounts {
            verified: 0,
            sourced: 0,
            inferred: 1,
            unknown: 0,
        },
        progress: Some(super::ProgressReport {
            done: vec!["a.txt".to_owned()],
            not_done: vec!["b.txt".to_owned()],
            stuck: super::ReasonPayload {
                code: "report-stuck-over-budget".to_owned(),
                params: Vec::new(),
            },
            next: super::ReasonPayload {
                code: "report-next-raise-limit".to_owned(),
                params: Vec::new(),
            },
        }),
    };
    let text = serde_json::to_string(&report).unwrap();
    let back: super::DeliveryReportRecord = serde_json::from_str(&text).unwrap();
    assert_eq!(back, report);
    let relayed = crate::EvidencePayload::Relayed {
        from_bot: "Bot2".to_owned(),
        text_head: "ok".to_owned(),
    };
    let r_text = serde_json::to_string(&relayed).unwrap();
    let r_back: crate::EvidencePayload = serde_json::from_str(&r_text).unwrap();
    assert_eq!(r_back, relayed);
}
