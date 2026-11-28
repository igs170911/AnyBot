use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, task_of};
use runner_test::turn_tool_helpers::{call, intent_call_ids, one_tool_step, tool_message_content};

#[tokio::test]
async fn t_files_edit_5() -> Result<(), String> {
    let lines: Vec<String> = (1..=3_000).map(|n| format!("line-{n}")).collect();
    let setup = WorldSetup {
        files: vec![("big.txt".to_owned(), lines.join("\n"))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "read_file",
        "{\"path\":\"big.txt\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "read big").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("no call id")?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), first_id)?;
    assert!(got.contains("1: line-1"));
    assert!(got.contains("2000: line-2000"));
    assert!(!got.contains("line-2001"));
    let wanted_more = fmt_text(
        Lang::En,
        "tool-read-more",
        &Params::new()
            .with_text("remaining", "1000")
            .with_text("next_offset", "2000"),
    )?;
    assert!(got.ends_with(&wanted_more));
    Ok(())
}

#[tokio::test]
async fn t_files_edit_6() -> Result<(), String> {
    let huge = "a".repeat(9_437_184);
    let mut bin = "a".repeat(7_999);
    bin.push('\u{0}');
    bin.push_str("tail");
    let setup = WorldSetup {
        files: vec![("huge.txt".to_owned(), huge), ("bin".to_owned(), bin)],
        dirs: vec!["d".to_owned()],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        call("read_file", "{\"path\":\"huge.txt\"}", "c1"),
        call("read_file", "{\"path\":\"d\"}", "c2"),
        call("read_file", "{\"path\":\"bin\"}", "c3"),
    ]));
    let outcome = world.send(Device::Phone, "read three").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let got1 = tool_message_content(&messages, ids.first().map(String::as_str).ok_or("id 1")?)?;
    let got2 = tool_message_content(&messages, ids.get(1).map(String::as_str).ok_or("id 2")?)?;
    let got3 = tool_message_content(&messages, ids.get(2).map(String::as_str).ok_or("id 3")?)?;
    let want1 = fmt_text(
        Lang::En,
        "tool-read-too-big",
        &Params::new().with_text("path", "huge.txt"),
    )?;
    let want2 = fmt_text(
        Lang::En,
        "tool-read-directory",
        &Params::new().with_text("path", "d"),
    )?;
    let want3 = fmt_text(
        Lang::En,
        "tool-read-not-text",
        &Params::new().with_text("path", "bin"),
    )?;
    assert_eq!(got1, want1);
    assert_eq!(got2, want2);
    assert_eq!(got3, want3);
    Ok(())
}
