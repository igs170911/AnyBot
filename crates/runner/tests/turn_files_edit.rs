use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::KnownTaskEventKind;
use runner_test::turn::{Device, TurnWorld};
use runner_test::turn_helpers::{
    fmt_text, kind_names, messages_of, request_body, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{
    edit_call, intent_call_ids, one_tool_step, tool_message_content, world_with,
};

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
