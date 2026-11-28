use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{ComputerCaps, ComputerOwner, ComputerPlatform, KnownTaskEventKind};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    fmt_text, messages_of, request_body, sha256_hex, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{call, one_tool_step, tool_message_content};
use test_harness::{model_replies_text, model_tool_calls};

fn expected_result_id(text: &str) -> String {
    let hex = sha256_hex(text.as_bytes());
    let prefix: String = hex.chars().take(16).collect();
    format!("r{prefix}")
}

fn call_ids_for_tool(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    tool: &str,
) -> Vec<String> {
    let mut ids = Vec::new();
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::ToolIntent { call_id, name, .. } = kind
            && name == tool
        {
            ids.push(call_id.as_str());
        }
    }
    ids
}

#[tokio::test]
async fn t_files_n1_1() -> Result<(), String> {
    let output = "0123456789".repeat(5_000);
    let id = expected_result_id(&output);
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        scripts: vec![("echo big".to_owned(), CommandScript::ok(&output))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let read_args = format!(r#"{{"id":"{id}","from":6000,"chars":30000}}"#);
    world.script(vec![
        model_tool_calls(
            vec![call("shell", r#"{"command":"echo big"}"#, "c1")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("read_result", &read_args, "c2")],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "read slice").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let read_ids = call_ids_for_tool(&world, &task, "read_result");
    let read_call_id = read_ids.first().ok_or("no read_result call id")?;
    let body = request_body(&world.model, 2)?;
    let content = tool_message_content(&messages_of(&body), read_call_id)?;
    let header = fmt_text(
        Lang::ZhTw,
        "tool-read-result-window",
        &Params::new()
            .with_text("from", "6000")
            .with_text("to", "26000")
            .with_text("total", "50000"),
    )?;
    let expected_slice = output.get(6_000..26_000).ok_or("slice out of bounds")?;
    assert!(content.starts_with(&header), "got: {content}");
    assert_eq!(content.lines().last(), Some(expected_slice));
    let has_gate = task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::GateResult { call_id, .. }
            if &call_id.as_str() == read_call_id
        )
    });
    assert!(!has_gate);
    let evidence_count = task_kinds(&world, &task)
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::EvidenceAdded { .. }))
        .count();
    assert_eq!(evidence_count, 1);
    Ok(())
}

#[tokio::test]
async fn t_files_n1_6() -> Result<(), String> {
    let output = "a".repeat(26_214_400);
    let kept_len = 20 * 1024 * 1024;
    let kept = output.get(..kept_len).ok_or("kept out of bounds")?;
    let id = expected_result_id(kept);
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        scripts: vec![("big25m".to_owned(), CommandScript::ok(&output))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let read_args = format!(r#"{{"id":"{id}","from":20000000}}"#);
    world.script(vec![
        model_tool_calls(
            vec![call("shell", r#"{"command":"big25m"}"#, "c1")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("read_result", &read_args, "c2")],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "read tail of 25mb").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let read_ids = call_ids_for_tool(&world, &task, "read_result");
    let read_call_id = read_ids.first().ok_or("no read_result call id")?;
    let body = request_body(&world.model, 2)?;
    let content = tool_message_content(&messages_of(&body), read_call_id)?;
    let cut_note = fmt_text(
        Lang::ZhTw,
        "tool-read-result-cut",
        &Params::new().with_text("mb", "20"),
    )?;
    assert!(content.contains(&cut_note), "got: {content}");
    assert!(content.contains(&"a".repeat(20_000)));
    Ok(())
}

fn expected_hit(at: &str, before: char, after: char) -> Result<String, String> {
    let context = format!(
        "{}ERROR 42{}",
        before.to_string().repeat(200),
        after.to_string().repeat(200)
    );
    fmt_text(
        Lang::ZhTw,
        "tool-read-result-hit",
        &Params::new()
            .with_text("at", at)
            .with_text("context", &context),
    )
}

#[tokio::test]
async fn t_files_n1_7() -> Result<(), String> {
    let output = format!(
        "{}ERROR 42{}ERROR 42{}",
        "a".repeat(10_000),
        "b".repeat(29_992),
        "c".repeat(9_992)
    );
    assert_eq!(output.chars().count(), 50_000);
    let id = expected_result_id(&output);
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        scripts: vec![("scan".to_owned(), CommandScript::ok(&output))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let find_err = serde_json::json!({ "id": id, "find": "ERROR 42" }).to_string();
    let find_none =
        serde_json::json!({ "id": id, "find": "\u{6c92}\u{6709}\u{7684}\u{5b57}" }).to_string();
    world.script(vec![
        model_tool_calls(
            vec![call("shell", r#"{"command":"scan"}"#, "c1")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("read_result", &find_err, "c2")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("read_result", &find_none, "c3")],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "find errors").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let read_ids = call_ids_for_tool(&world, &task, "read_result");
    let first_id = read_ids.first().ok_or("no first read_result call id")?;
    let second_id = read_ids.get(1).ok_or("no second read_result call id")?;
    let found_text = tool_message_content(&messages_of(&request_body(&world.model, 2)?), first_id)?;
    assert!(found_text.contains(&expected_hit("10000", 'a', 'b')?));
    assert!(found_text.contains(&expected_hit("40000", 'b', 'c')?));
    let missing_text =
        tool_message_content(&messages_of(&request_body(&world.model, 3)?), second_id)?;
    assert_eq!(
        missing_text,
        fmt_text(Lang::ZhTw, "tool-read-result-not-found", &Params::new())?
    );
    Ok(())
}

#[tokio::test]
async fn unknown_id_is_missing() -> Result<(), String> {
    let caps = ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::User);
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        caps: Some(caps),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "read_result",
        r#"{"id":"r-bogus"}"#,
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "read missing").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = call_ids_for_tool(&world, &task, "read_result");
    let call_id = ids.first().ok_or("no call id")?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), call_id)?;
    let wanted = fmt_text(Lang::ZhTw, "tool-read-result-missing", &Params::new())?;
    assert_eq!(got, wanted);
    assert!(world.gated.logged().is_empty());
    Ok(())
}
