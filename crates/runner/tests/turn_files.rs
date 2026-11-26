use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use sha2 as _;
use thiserror as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::KnownTaskEventKind;
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup, tool_spec};
use runner_test::turn_helpers::{
    fmt_text, kind_names, messages_of, request_body, sha256_hex, task_kinds, task_of,
};
use test_harness::{ToolCallScript, model_replies_text, model_tool_calls};

fn all_specs() -> Vec<anybot_models::ToolSpec> {
    vec![
        tool_spec("edit_file", "Edit a file"),
        tool_spec("read_file", "Read a file"),
        tool_spec("shell", "Run a shell command"),
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

fn edit_call(args: &str, id: &str) -> ToolCallScript {
    call("edit_file", args, id)
}

fn one_tool_step(calls: Vec<ToolCallScript>) -> Vec<test_harness::Step> {
    vec![
        model_tool_calls(calls, "tool_calls", false),
        model_replies_text("done", false),
    ]
}

async fn world_with(files: Vec<(String, String)>) -> Result<TurnWorld, String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files,
        ..WorldSetup::default()
    };
    TurnWorld::start(&setup).await
}

#[tokio::test]
async fn t_computer_ops_2() -> Result<(), String> {
    let world = world_with(Vec::new()).await?;
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let record = evidence_record(&world, &task, "a.txt")?;
    let anybot_protocol::EvidencePayload::Changed { before, after, .. } = &record.payload else {
        return Err("not a changed payload".to_owned());
    };
    if !matches!(before, anybot_protocol::BeforeDigest::Absent) {
        return Err("before is not absent".to_owned());
    }
    let wanted = sha256_hex(b"x");
    if after.hex() != wanted {
        return Err(format!("after {} wanted {}", after.hex(), wanted));
    }
    Ok(())
}

fn evidence_record(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    path: &str,
) -> Result<anybot_protocol::EvidenceRecord, String> {
    for kind in task_kinds(world, task) {
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

#[tokio::test]
async fn t_files_shell_4() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![(
            "echo oops; exit 2".to_owned(),
            CommandScript::failing("oops\n", 2),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"echo oops; exit 2\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let record = exit_evidence(&world, &task)?;
    let anybot_protocol::EvidencePayload::Executed {
        exit_code: Some(2), ..
    } = &record.payload
    else {
        return Err("exit code is not 2".to_owned());
    };
    check_model_sees_exit_and_output(&world, &task)?;
    Ok(())
}

fn exit_evidence(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<anybot_protocol::EvidenceRecord, String> {
    for kind in task_kinds(world, task) {
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

fn check_model_sees_exit_and_output(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<(), String> {
    let body = request_body(&world.model, 1)?;
    let ids = intent_call_ids(world, task);
    let content = tool_message_content(
        &messages_of(&body),
        ids.first()
            .map(String::as_str)
            .ok_or("no first id".to_owned())?,
    )?;
    let wanted_code = fmt_text(
        Lang::En,
        "tool-shell-failed",
        &Params::new().with_text("code", "2"),
    )?;
    if !content.contains(&wanted_code) {
        return Err(format!("no exit text in {content}"));
    }
    if !content.contains("oops") {
        return Err(format!("no output text in {content}"));
    }
    Ok(())
}

fn intent_call_ids(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Vec<String> {
    let mut ids = Vec::new();
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::ToolIntent { call_id, .. } = kind {
            ids.push(call_id.as_str());
        }
    }
    ids
}

fn tool_message_content(messages: &[serde_json::Value], call_id: &str) -> Result<String, String> {
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

#[tokio::test]
async fn t_files_edit_7() -> Result<(), String> {
    let world = world_with(vec![("a.txt".to_owned(), "x and x".to_owned())]).await?;
    world.script(one_tool_step(vec![
        edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"x\",\"new_text\":\"y\"}",
            "c1",
        ),
        edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"zzz\",\"new_text\":\"q\"}",
            "c2",
        ),
    ]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let multiple = fmt_text(
        Lang::En,
        "tool-edit-multiple",
        &Params::new()
            .with_text("path", "a.txt")
            .with_text("count", "2"),
    )?;
    let missing = fmt_text(
        Lang::En,
        "tool-edit-not-found",
        &Params::new().with_text("path", "a.txt"),
    )?;
    let ids = intent_call_ids(&world, &task);
    let first = tool_message_content(
        &messages,
        ids.first()
            .map(String::as_str)
            .ok_or("no first id".to_owned())?,
    )?;
    let second = tool_message_content(
        &messages,
        ids.get(1)
            .map(String::as_str)
            .ok_or("no second id".to_owned())?,
    )?;
    if !first.contains(&multiple) {
        return Err(format!("c1 missing {multiple} got {first}"));
    }
    if !second.contains(&missing) {
        return Err(format!("c2 missing {missing} got {second}"));
    }
    if world.tree_content("a.txt").as_deref() != Some("x and x") {
        return Err("a.txt changed".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn t_files_edit_8() -> Result<(), String> {
    let world = world_with(Vec::new()).await?;
    world.script(one_tool_step(vec![
        edit_call(
            "{\"path\":\"d/e/n.txt\",\"old_text\":\"\",\"new_text\":\"1\\n2\"}",
            "c1",
        ),
        edit_call(
            "{\"path\":\"d/e/n.txt\",\"old_text\":\"1\",\"new_text\":\"3\"}",
            "c2",
        ),
    ]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let created = fmt_text(
        Lang::En,
        "tool-edit-created",
        &Params::new()
            .with_text("path", "d/e/n.txt")
            .with_text("lines", "2"),
    )?;
    let modified = fmt_text(
        Lang::En,
        "tool-edit-modified",
        &Params::new().with_text("path", "d/e/n.txt"),
    )?;
    let ids = intent_call_ids(&world, &task);
    let first = tool_message_content(
        &messages,
        ids.first()
            .map(String::as_str)
            .ok_or("no first id".to_owned())?,
    )?;
    let second = tool_message_content(
        &messages,
        ids.get(1)
            .map(String::as_str)
            .ok_or("no second id".to_owned())?,
    )?;
    if !first.contains(&created) {
        return Err(format!("c1 missing {created} got {first}"));
    }
    if !second.contains(&modified) {
        return Err(format!("c2 missing {modified} got {second}"));
    }
    check_two_hash_pairs(&world, &task)?;
    Ok(())
}

fn check_two_hash_pairs(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let mut pairs = 0usize;
    for kind in task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if matches!(
            &record.payload,
            anybot_protocol::EvidencePayload::Changed { .. }
        ) {
            pairs = pairs.saturating_add(1);
        }
    }
    if pairs != 2 {
        return Err(format!("pairs {pairs}"));
    }
    if !kind_names(world, task).contains(&"tool_started".to_owned()) {
        return Err("no tool_started".to_owned());
    }
    Ok(())
}
