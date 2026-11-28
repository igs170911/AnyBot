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
use test_harness as _;
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{EvidenceKind, KnownTaskEventKind};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, task_kinds, task_of};
use runner_test::turn_tool_helpers::{
    call, first_id, intent_call_ids, one_tool_step, tool_message_content,
};

#[tokio::test]
async fn t_files_edit_3() -> Result<(), String> {
    let setup = WorldSetup {
        ripgrep: true,
        files: vec![
            ("a.txt".to_owned(), "foo123\nbar\n".to_owned()),
            (".env".to_owned(), "foo9\n".to_owned()),
        ],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "search",
        r#"{"pattern":"foo[0-9]+"}"#,
        "s1",
    )]));
    let outcome = world.send(Device::Phone, "search regex").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let body = request_body(&world.model, 1)?;
    let content = tool_message_content(&messages_of(&body), first_id(&ids)?)?;
    assert!(content.contains("a.txt:1:foo123"), "got: {content}");
    assert!(!content.contains(".env"), "got: {content}");
    assert!(!content.contains("foo9"), "got: {content}");
    let seen = task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::EvidenceAdded { record }
            if record.kind == EvidenceKind::Seen
        )
    });
    assert!(seen);
    Ok(())
}

#[tokio::test]
async fn t_files_edit_4() -> Result<(), String> {
    let setup = WorldSetup {
        ripgrep: false,
        files: vec![
            ("a.txt".to_owned(), "FOO\nbar\n".to_owned()),
            ("node_modules/x.js".to_owned(), "foo\n".to_owned()),
            (".env".to_owned(), "foo\n".to_owned()),
        ],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "search",
        r#"{"pattern":"foo"}"#,
        "s1",
    )]));
    let outcome = world.send(Device::Phone, "search plain").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let body = request_body(&world.model, 1)?;
    let content = tool_message_content(&messages_of(&body), first_id(&ids)?)?;
    assert!(content.contains("a.txt:1:FOO"), "got: {content}");
    assert!(!content.contains("node_modules"), "got: {content}");
    assert!(!content.contains(".env"), "got: {content}");
    Ok(())
}

#[tokio::test]
async fn search_files_only_lists_matching_paths_and_empty_reports_missing() -> Result<(), String> {
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        files: vec![
            (
                "a.txt".to_owned(),
                "needle line 1\nneedle line 2\n".to_owned(),
            ),
            ("b.txt".to_owned(), "needle here\n".to_owned()),
            ("c.txt".to_owned(), "nothing\n".to_owned()),
        ],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        call("search", r#"{"pattern":"needle","files_only":true}"#, "s1"),
        call("search", r#"{"pattern":"absent"}"#, "s2"),
    ]));
    let outcome = world.send(Device::Phone, "search files only").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let body = request_body(&world.model, 1)?;
    let msgs = messages_of(&body);
    let first = tool_message_content(&msgs, first_id(&ids)?)?;
    assert_eq!(first, "a.txt\nb.txt");
    let second = tool_message_content(&msgs, ids.get(1).ok_or("no second id")?)?;
    let empty_msg = fmt_text(Lang::ZhTw, "tool-search-empty", &Params::new())?;
    assert_eq!(second, empty_msg);
    Ok(())
}

#[tokio::test]
async fn search_truncates_at_two_thousand_and_keeps_total_found() -> Result<(), String> {
    use std::fmt::Write as _;

    let mut big = String::new();
    for i in 1..=12_000 {
        writeln!(&mut big, "match_{i}").map_err(|e| e.to_string())?;
    }
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        files: vec![("big.txt".to_owned(), big)],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "search",
        r#"{"pattern":"match_"}"#,
        "s1",
    )]));
    let outcome = world.send(Device::Phone, "search big").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let body = request_body(&world.model, 1)?;
    let content = tool_message_content(&messages_of(&body), first_id(&ids)?)?;
    let cut_note = fmt_text(
        Lang::ZhTw,
        "tool-search-cut",
        &Params::new()
            .with_text("shown", "2000")
            .with_text("found", "10000"),
    )?;
    assert!(content.contains("big.txt:1:match_1"));
    assert!(content.contains("big.txt:2000:match_2000"));
    assert!(!content.contains("big.txt:2001:match_2001"));
    assert!(content.contains(&cut_note), "got: {content}");
    Ok(())
}
