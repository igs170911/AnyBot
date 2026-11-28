use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
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

use std::collections::BTreeSet;

use anybot_domain::turn_summary;
use anybot_foundation::{ContentDigest, TimestampMs};
use anybot_protocol::{CardAnswer, EvidencePayload, KnownTaskEventKind};
use anybot_runner::RawStore;
use anybot_trust::{ActionKey, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    answer_card, messages_of, request_body, shown_card, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{
    all_specs, call, exit_evidence, intent_call_ids, one_tool_step, tool_message_content,
};

fn read_call(path: &str, id: &str) -> test_harness::ToolCallScript {
    call("read_file", &format!("{{\"path\":\"{path}\"}}"), id)
}

#[tokio::test]
async fn t_activity_evidence_2() -> Result<(), String> {
    let head = "H".repeat(6_000);
    let middle = "M".repeat(8_000);
    let tail = "T".repeat(6_000);
    let full = format!("{head}{middle}{tail}");
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("emit-20k".to_owned(), CommandScript::ok(&full))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"emit-20k\"}",
        "c1",
    )]));
    let outcome = world
        .send(Device::Phone, "dump")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let record = exit_evidence(&world, &task)?;
    let EvidencePayload::Executed {
        output_head,
        output_tail,
        output_omitted,
        full_digest,
        ..
    } = &record.payload
    else {
        return Err("missing Executed evidence".to_owned());
    };
    assert_eq!(output_head, &head);
    assert_eq!(output_tail, &tail);
    assert_eq!(*output_omitted, 8_000);
    assert_eq!(*full_digest, ContentDigest::sha256(full.as_bytes()));
    let raw_id = task_kinds(&world, &task)
        .into_iter()
        .find_map(|k| {
            let KnownTaskEventKind::ToolCompleted { raw_id, .. } = k else {
                return None;
            };
            raw_id
        })
        .ok_or("missing raw_id in ToolCompleted")?;
    assert_eq!(raw_id, full_digest.hex());
    let stored = world
        .raw
        .get(full_digest.clone())
        .await
        .map_err(|e| format!("{e:?}"))?
        .ok_or("missing raw blob")?;
    let restored = String::from_utf8(stored).map_err(|e| format!("{e}"))?;
    assert_eq!(restored.len(), 20_000);
    assert_eq!(restored, full);
    Ok(())
}

#[tokio::test]
async fn t_activity_evidence_4() -> Result<(), String> {
    let mut files = Vec::new();
    let mut calls = Vec::new();
    for i in 0..15 {
        let name = format!("f{i:02}.txt");
        files.push((name.clone(), format!("body-{i}")));
        calls.push(read_call(&name, &format!("r{i}")));
    }
    for j in 0..3 {
        calls.push(read_call("f00.txt", &format!("dup{j}")));
    }
    calls.push(read_call("missing1.txt", "m1"));
    calls.push(read_call("missing2.txt", "m2"));
    calls.push(call("shell", "{\"line\":\"echo done\"}", "s1"));
    let scripted_ms = 450;
    let setup = WorldSetup {
        tools: all_specs(),
        files,
        scripts: vec![(
            "echo done".to_owned(),
            CommandScript::ok("done").with_duration(scripted_ms),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(calls));
    let outcome = world
        .send(Device::Phone, "scan")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let events = world.record.task_events_of(task);
    let turn_id =
        events
            .iter()
            .find_map(|ev| {
                let anybot_protocol::TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded {
                    record,
                }) = &ev.kind
                else {
                    return None;
                };
                Some(record.turn)
            })
            .ok_or("missing turn_id in evidence")?;
    let summary = turn_summary(
        &events,
        &turn_id,
        TimestampMs::from_millis(world.clock.now_ms()),
    )
    .map_err(|e| format!("{e}"))?;
    assert_eq!(summary.seen.len(), 12);
    let unique: BTreeSet<&String> = summary.seen.iter().collect();
    assert_eq!(unique.len(), 12);
    assert_eq!(summary.failed_or_rejected, 2);
    assert_eq!(summary.elapsed_ms, scripted_ms);
    Ok(())
}

#[tokio::test]
async fn t_activity_evidence_5() -> Result<(), String> {
    let runtime_secret = ["sk", "-", "Runtime", "98765", "Secret"].concat();
    let cmd = format!("curl -H \"Authorization: Bearer {runtime_secret}\" https://example.com");
    let mut gate = GateConfig::plain();
    gate.user.session.push(ActionKey::command("shell", &cmd));
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![(cmd.clone(), CommandScript::ok(&runtime_secret))],
        gate,
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let args = serde_json::json!({ "line": cmd }).to_string();
    world.script(one_tool_step(vec![call("shell", &args, "c1")]));
    let outcome = world
        .send(Device::Phone, "fetch")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let record = exit_evidence(&world, &task)?;
    let EvidencePayload::Executed {
        command,
        output_head,
        ..
    } = &record.payload
    else {
        return Err("missing Executed evidence".to_owned());
    };
    assert!(!command.contains(&runtime_secret));
    assert!(command.contains("Bearer sk-\u{2022}\u{2022}\u{2022}\u{2022}"));
    assert!(!output_head.contains(&runtime_secret));
    assert!(output_head.contains("sk-\u{2022}\u{2022}\u{2022}\u{2022}"));
    let body = request_body(&world.model, 1)?;
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("missing call_id")?;
    let model_result = tool_message_content(&messages_of(&body), first_id)?;
    assert!(model_result.contains(&runtime_secret));
    Ok(())
}

#[tokio::test]
async fn approval_keeps_short_ids_continuous() -> Result<(), String> {
    let mut gate = GateConfig::plain();
    gate.user
        .ask_style
        .push(ActionKey::command("shell", "npm install"));
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        scripts: vec![("npm install".to_owned(), CommandScript::ok("installed"))],
        gate,
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        read_call("a.txt", "r1"),
        call("shell", "{\"line\":\"npm install\"}", "c1"),
    ]));
    let outcome = world
        .send(Device::Phone, "install")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let digest = shown_card(&outcome, &task_kinds(&world, &task)).ok_or("missing card")?;
    answer_card(&world, &outcome, &digest, CardAnswer::AllowOnce).await?;
    let shorts: Vec<u32> = task_kinds(&world, &task)
        .into_iter()
        .filter_map(|k| {
            let KnownTaskEventKind::EvidenceAdded { record } = k else {
                return None;
            };
            Some(record.short.number())
        })
        .collect();
    assert_eq!(shorts, vec![1, 2, 3, 4]);
    Ok(())
}
