use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{EvidenceKind, EvidencePayload, KnownTaskEventKind};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, intent_call_ids, one_tool_step, tool_message_content};

#[tokio::test]
async fn t_files_edit_1() -> Result<(), String> {
    let setup = WorldSetup {
        files: vec![
            ("A.txt".to_owned(), "abc".to_owned()),
            ("c.txt".to_owned(), String::new()),
            (".env".to_owned(), "SECRET=1".to_owned()),
        ],
        dirs: vec!["b".to_owned()],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "list_dir",
        "{\"path\":\".\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "list").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("no call id")?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), first_id)?;
    assert_eq!(got, "b/\nA.txt (3 B)\nc.txt (0 B)");
    Ok(())
}

#[tokio::test]
async fn t_files_edit_2() -> Result<(), String> {
    let files: Vec<(String, String)> = (1..=2_004)
        .map(|n| (format!("f{n:04}.txt"), String::new()))
        .collect();
    let setup = WorldSetup {
        files,
        dirs: vec!["e".to_owned()],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        call("list_dir", "{\"path\":\".\"}", "c1"),
        call("list_dir", "{\"path\":\"e\"}", "c2"),
    ]));
    let outcome = world.send(Device::Phone, "list many").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let ids = intent_call_ids(&world, &task);
    let id1 = ids.first().map(String::as_str).ok_or("no id 1")?;
    let id2 = ids.get(1).map(String::as_str).ok_or("no id 2")?;

    let summary1 = completed_summary(&world, &task, id1)?;
    let summary_lines: Vec<&str> = summary1.lines().collect();
    assert_eq!(summary_lines.len(), 2_001);
    assert_eq!(summary_lines.first().copied(), Some("e/"));
    assert_eq!(summary_lines.get(1999).copied(), Some("f1999.txt (0 B)"));
    assert!(!summary1.contains("f2000.txt"));

    let wanted_more = fmt_text(
        Lang::En,
        "tool-list-more",
        &Params::new().with_text("count", "5"),
    )?;
    assert_eq!(summary_lines.last().copied(), Some(wanted_more.as_str()));

    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    let got1 = tool_message_content(&messages, id1)?;
    assert!(got1.starts_with("e/\n"));
    assert!(got1.contains("f1999.txt (0 B)"));
    assert!(!got1.contains("f2000.txt"));
    assert!(got1.ends_with(&wanted_more));

    let got2 = tool_message_content(&messages, id2)?;
    let wanted_empty = fmt_text(Lang::En, "tool-list-empty", &Params::new())?;
    assert_eq!(got2, wanted_empty);
    Ok(())
}

#[tokio::test]
async fn list_dir_is_read_only_in_gate() -> Result<(), String> {
    let world = TurnWorld::start(&WorldSetup::default()).await?;
    world.script(one_tool_step(vec![call(
        "list_dir",
        "{\"path\":\"/home/bot/bot-2\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "list other").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let denied = task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::EvidenceAdded { record }
            if record.kind == EvidenceKind::Rejected
                && matches!(
                    &record.payload,
                    EvidencePayload::Rejected { reason }
                    if reason.code == "gate-bot-machine-outside"
                )
        )
    });
    assert!(denied);
    assert!(world.gated.logged().is_empty());
    Ok(())
}

#[tokio::test]
async fn list_dir_on_file_reports_operation_failure() -> Result<(), String> {
    let setup = WorldSetup {
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "list_dir",
        "{\"path\":\"a.txt\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "list file").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let failed = task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::ToolFailed { error, .. }
            if error.contains("is a file")
        )
    });
    assert!(failed);
    Ok(())
}

fn completed_summary(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    call_id: &str,
) -> Result<String, String> {
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::ToolCompleted {
            call_id: got_id,
            summary,
            ..
        } = kind
            && got_id.as_str() == call_id
        {
            return Ok(summary);
        }
    }
    Err(format!("missing ToolCompleted for {call_id}"))
}
