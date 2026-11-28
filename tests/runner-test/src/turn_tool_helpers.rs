use anybot_messages::Lang;
use anybot_protocol::KnownTaskEventKind;
use runner_test::turn::{TurnWorld, WorldSetup};

use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

#[must_use]
pub fn call(name: &str, args: &str, id: &str) -> ToolCallScript {
    ToolCallScript {
        name: name.to_owned(),
        arguments: args.to_owned(),
        id: Some(id.to_owned()),
        argument_parts: Vec::new(),
    }
}

#[must_use]
pub fn edit_call(args: &str, id: &str) -> ToolCallScript {
    call("edit_file", args, id)
}

#[must_use]
pub fn one_tool_step(calls: Vec<ToolCallScript>) -> Vec<Step> {
    vec![
        model_tool_calls(calls, "tool_calls", false),
        model_replies_text("done", false),
    ]
}

pub async fn world_with(files: Vec<(String, String)>) -> Result<TurnWorld, String> {
    let setup = WorldSetup {
        files,
        ..WorldSetup::default()
    };
    TurnWorld::start(&setup).await
}

pub fn evidence_record(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    path: &str,
) -> Result<anybot_protocol::EvidenceRecord, String> {
    for kind in runner_test::turn_helpers::task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if let anybot_protocol::EvidencePayload::Changed { path: got, .. } = &record.payload
            && got.ends_with(path)
        {
            return Ok(*record);
        }
    }
    Err("no evidence for the path".to_owned())
}

pub fn exit_evidence(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<anybot_protocol::EvidenceRecord, String> {
    for kind in runner_test::turn_helpers::task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if matches!(
            &record.payload,
            anybot_protocol::EvidencePayload::Executed { .. }
        ) {
            return Ok(*record);
        }
    }
    Err("no executed evidence".to_owned())
}

#[must_use]
pub fn intent_call_ids(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Vec<String> {
    let mut ids = Vec::new();
    for kind in runner_test::turn_helpers::task_kinds(world, task) {
        if let KnownTaskEventKind::ToolIntent { call_id, .. } = kind {
            ids.push(call_id.as_str());
        }
    }
    ids
}

pub fn tool_message_content(
    messages: &[serde_json::Value],
    call_id: &str,
) -> Result<String, String> {
    for item in messages {
        if item.get("tool_call_id").and_then(|v| v.as_str()) == Some(call_id) {
            let content = item.get("content").and_then(|v| v.as_str());
            let Some(text) = content else {
                return Err("tool message without content".to_owned());
            };
            return Ok(text.to_owned());
        }
    }
    Err("no tool message for the call".to_owned())
}

pub fn first_id(ids: &[String]) -> Result<&str, String> {
    ids.first()
        .map(String::as_str)
        .ok_or("no first id".to_owned())
}

pub fn second_id(ids: &[String]) -> Result<&str, String> {
    ids.get(1)
        .map(String::as_str)
        .ok_or("no second id".to_owned())
}

pub const LANG: Lang = Lang::En;
