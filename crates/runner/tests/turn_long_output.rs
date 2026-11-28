use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;
use tokio as _;

use std::fmt::Write as _;

use anybot_messages::{Lang, Params};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, sha256_hex, task_of};
use runner_test::turn_tool_helpers::{call, intent_call_ids, one_tool_step, tool_message_content};

fn expected_result_id(output: &str) -> String {
    let hex = sha256_hex(output.as_bytes());
    let prefix: String = hex.chars().take(16).collect();
    format!("r{prefix}")
}

fn take_last(text: &str, count: usize) -> String {
    let chars: Vec<char> = text.chars().rev().take(count).collect();
    chars.into_iter().rev().collect()
}

fn build_seq_output(max: usize) -> Result<String, String> {
    let mut output = String::with_capacity(600_000);
    for n in 1..=max {
        writeln!(output, "{n}").map_err(|e| e.to_string())?;
    }
    Ok(output)
}

#[tokio::test]
async fn t_files_shell_3() -> Result<(), String> {
    let output = build_seq_output(100_000)?;
    assert_eq!(output.chars().count(), 588_895);
    let setup = WorldSetup {
        scripts: vec![("seq 1 100000".to_owned(), CommandScript::ok(&output))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"command\":\"seq 1 100000\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "count").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let call_id = ids.first().map(String::as_str).ok_or("no call id")?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), call_id)?;
    let expected_head: String = output.chars().take(6_000).collect();
    let expected_tail = take_last(&output, 6_000);
    let expected_id = expected_result_id(&output);
    let wanted_marker = fmt_text(
        Lang::En,
        "tool-shell-trimmed",
        &Params::new()
            .with_text("omitted", "576895")
            .with_text("id", &expected_id),
    )?;
    assert!(got.starts_with(&expected_head));
    assert!(got.ends_with(&expected_tail));
    assert!(got.contains(&wanted_marker));
    Ok(())
}

#[tokio::test]
async fn t_files_n1_2() -> Result<(), String> {
    let output = "0123456789".repeat(5_000);
    let setup = WorldSetup {
        scripts: vec![("echo big".to_owned(), CommandScript::ok(&output))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"command\":\"echo big\"}",
        "c1",
    )]));
    let outcome1 = world.send(Device::Phone, "first").await;
    let task1 = task_of(&outcome1.map_err(|e| format!("{e:?}"))?);
    let id1 = intent_call_ids(&world, &task1)
        .into_iter()
        .next()
        .ok_or("no id 1")?;
    let msg1 = tool_message_content(&messages_of(&request_body(&world.model, 1)?), &id1)?;

    world.script(one_tool_step(vec![call(
        "shell",
        "{\"command\":\"echo big\"}",
        "c2",
    )]));
    let outcome2 = world.send(Device::Phone, "second").await;
    let task2 = task_of(&outcome2.map_err(|e| format!("{e:?}"))?);
    let id2 = intent_call_ids(&world, &task2)
        .into_iter()
        .next()
        .ok_or("no id 2")?;
    let msg2 = tool_message_content(&messages_of(&request_body(&world.model, 3)?), &id2)?;

    let wanted_id = expected_result_id(&output);
    let wanted_marker = fmt_text(
        Lang::En,
        "tool-shell-trimmed",
        &Params::new()
            .with_text("omitted", "38000")
            .with_text("id", &wanted_id),
    )?;
    assert!(msg1.contains(&wanted_marker));
    assert!(msg2.contains(&wanted_marker));
    Ok(())
}

#[tokio::test]
async fn long_failing_shell_output_keeps_error_window_and_id() -> Result<(), String> {
    let output = format!("{}\nerror: boom\n{}", "a".repeat(8_000), "b".repeat(8_000));
    let setup = WorldSetup {
        scripts: vec![("build".to_owned(), CommandScript::failing(&output, 2))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"command\":\"build\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "build").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let call_id = intent_call_ids(&world, &task)
        .into_iter()
        .next()
        .ok_or("no call id")?;
    let got = tool_message_content(&messages_of(&request_body(&world.model, 1)?), &call_id)?;
    let wanted_id = expected_result_id(&output);
    let wanted_code = fmt_text(
        Lang::En,
        "tool-shell-failed",
        &Params::new().with_text("code", "2"),
    )?;
    assert!(got.contains(&wanted_code));
    assert!(got.contains("error: boom"));
    assert!(got.contains(&wanted_id));
    Ok(())
}
