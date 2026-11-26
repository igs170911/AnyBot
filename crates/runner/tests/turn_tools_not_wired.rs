use anybot_context as _;
use anybot_domain as _;
use anybot_runner as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_foundation::TaskId;
use anybot_messages::{Lang, Params};
use anybot_protocol::{KnownTaskEventKind, ToolRequest};
use runner_test::turn::{Device, TurnWorld, WorldSetup, tool_spec};
use runner_test::turn_helpers::{
    chat_requests, fmt_text, kind_names, messages_of, raw_has, state_of, task_kinds, task_of,
};
use test_harness::{
    ModelFormat, ToolCallScript, model_replies_text, model_think_then_tool, model_tool_calls,
};

fn two_specs() -> Vec<anybot_models::ToolSpec> {
    vec![
        tool_spec("read_file", "Read a file"),
        tool_spec("shell", "Run a shell command"),
        tool_spec("edit_file", "Edit a file"),
    ]
}

fn call(name: &str, args: &str, id: &str) -> ToolCallScript {
    ToolCallScript {
        name: name.to_owned(),
        arguments: args.to_owned(),
        id: Some(id.to_owned()),
        argument_parts: Vec::new(),
    }
}

#[tokio::test]
async fn turn_tool_requests_get_not_wired_results() -> Result<(), String> {
    let setup = WorldSetup {
        tools: two_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("shell", "{\"command\":\"ls\"}", "c2"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 2);
    assert_eq!(
        kind_names(&world, &task),
        [
            "task_created",
            "turn_started",
            "context_built",
            "model_asked",
            "model_answer",
            "model_usage",
            "tool_not_run",
            "tool_not_run",
            "context_built",
            "model_asked",
            "model_answer",
            "model_usage",
            "turn_ended"
        ]
    );
    check_call_ids(&world, &task)?;
    check_tool_messages(&world)?;
    assert!(!kind_names(&world, &task).contains(&"tool_started".to_owned()));
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, anybot_protocol::TaskStatus::Completed);
    Ok(())
}

fn check_call_ids(world: &TurnWorld, task: &TaskId) -> Result<(), String> {
    let kinds = task_kinds(world, task);
    let Some(KnownTaskEventKind::ModelAnswer { tool_requests, .. }) = kinds.get(4) else {
        return Err("no model_answer".to_owned());
    };
    assert_eq!(tool_requests.len(), 2);
    let mut not_run: Vec<anybot_foundation::CallId> = Vec::new();
    for kind in kinds.iter().skip(6).take(2) {
        let KnownTaskEventKind::ToolNotRun { call_id, .. } = kind else {
            return Err("not a tool_not_run".to_owned());
        };
        not_run.push(*call_id);
    }
    let wanted: Vec<anybot_foundation::CallId> = tool_requests
        .iter()
        .map(|r: &ToolRequest| r.call_id)
        .collect();
    assert_eq!(not_run, wanted);
    assert_eq!(
        tool_requests.first().map(|r| r.provider_call_id.as_str()),
        Some("c1")
    );
    assert_eq!(
        tool_requests.get(1).map(|r| r.provider_call_id.as_str()),
        Some("c2")
    );
    Ok(())
}

fn check_tool_messages(world: &TurnWorld) -> Result<(), String> {
    let body = runner_test::turn_helpers::request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let mut ids: Vec<String> = Vec::new();
    for item in &messages {
        if item.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }
        let Some(calls) = item.get("tool_calls").and_then(|v| v.as_array()) else {
            continue;
        };
        for call in calls {
            let Some(id) = call.get("id").and_then(|v| v.as_str()) else {
                return Err("assistant call without id".to_owned());
            };
            ids.push(id.to_owned());
        }
    }
    assert_eq!(ids.len(), 2);
    let wanted = [not_wired("read_file")?, not_wired("shell")?];
    let mut contents: Vec<String> = Vec::new();
    for item in &messages {
        let Some(id) = item.get("tool_call_id").and_then(|v| v.as_str()) else {
            continue;
        };
        if !ids.iter().any(|known| known == id) {
            return Err("tool message id not in assistant calls".to_owned());
        }
        let Some(content) = item.get("content").and_then(|v| v.as_str()) else {
            return Err("tool message without content".to_owned());
        };
        contents.push(content.to_owned());
    }
    assert_eq!(contents, wanted);
    Ok(())
}

fn not_wired(tool: &str) -> Result<String, String> {
    let params = Params::new().with_text("tool", tool);
    fmt_text(Lang::En, "turn-tool-not-wired", &params)
}

#[tokio::test]
async fn turn_native_reply_kept_as_attachment() -> Result<(), String> {
    let setup = WorldSetup {
        format: ModelFormat::Anthropic,
        tools: two_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_think_then_tool(
            "plan",
            "sig-1",
            vec![call("read_file", "{\"path\":\"a.txt\"}", "c1")],
            false,
        ),
        model_replies_text("ok", false),
    ]);
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let Some(KnownTaskEventKind::ModelAnswer {
        thinking, native, ..
    }) = kinds.get(4)
    else {
        return Err("no model_answer".to_owned());
    };
    assert_eq!(thinking.clone(), Some("plan".to_owned()));
    let Some(got) = native.clone() else {
        return Err("no native ref".to_owned());
    };
    assert_eq!(got.provider, "Anthropic");
    assert_eq!(got.model, "claude-opus-5-5");
    assert!(raw_has(&world.raw, &got.blob));
    Ok(())
}

use anybot_context as _;
use anybot_domain as _;
use sha2 as _;
