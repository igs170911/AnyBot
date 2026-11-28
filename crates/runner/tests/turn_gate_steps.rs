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

use anybot_messages::{Lang, Params};
use anybot_protocol::{
    ComputerOwner, ComputerPlatform, EvidenceKind, EvidencePayload, EvidenceRecord, GateOutcome,
    GateRecord, GateStep, JudgementSource, KnownTaskEventKind, RuleLayer,
};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};

fn user_mac_setup() -> WorldSetup {
    WorldSetup {
        owner: Some(ComputerOwner::User),
        platform: ComputerPlatform::Mac,
        computer_name: "Work Mac".to_owned(),
        bot_name: "Bot1".to_owned(),
        scripts: vec![("git status".to_owned(), CommandScript::ok("On branch main"))],
        ..WorldSetup::default()
    }
}

fn completed_summary(kinds: &[KnownTaskEventKind]) -> Option<&str> {
    kinds.iter().find_map(|k| {
        if let KnownTaskEventKind::ToolCompleted { summary, .. } = k {
            Some(summary.as_str())
        } else {
            None
        }
    })
}

fn evidence_of_kind(kinds: &[KnownTaskEventKind], kind: EvidenceKind) -> Option<&EvidenceRecord> {
    kinds.iter().find_map(|k| {
        if let KnownTaskEventKind::EvidenceAdded { record } = k
            && record.kind == kind
        {
            Some(record.as_ref())
        } else {
            None
        }
    })
}

fn first_gate_result(kinds: &[KnownTaskEventKind]) -> Option<&GateRecord> {
    kinds.iter().find_map(|k| {
        if let KnownTaskEventKind::GateResult { gate, .. } = k {
            Some(gate)
        } else {
            None
        }
    })
}

#[tokio::test]
async fn t_safety_gates_7() -> Result<(), String> {
    let world = TurnWorld::start(&user_mac_setup()).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"git status\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "status").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(&world, &task);
    let summary = completed_summary(&kinds).ok_or("missing ToolCompleted")?;
    assert!(
        summary.starts_with("[Work Mac] "),
        "expected [Work Mac] prefix, got: {summary}"
    );
    let record = evidence_of_kind(&kinds, EvidenceKind::Executed).ok_or("missing Executed")?;
    assert_eq!(record.gate.gate, GateStep::DefaultRule);
    assert_eq!(record.gate.gate.number(), 8);
    assert_eq!(record.gate.layer, RuleLayer::Default);
    assert_eq!(record.gate.rule, "read-only-command");
    assert_eq!(record.gate.source, JudgementSource::Rule);
    assert_eq!(record.gate.outcome, GateOutcome::Allow);
    assert!(record.gate.read_only);
    Ok(())
}

fn assert_bot_outside_record(record: &EvidenceRecord) -> Result<(), String> {
    assert_eq!(record.gate.gate, GateStep::DefaultRule);
    assert_eq!(record.gate.gate.number(), 8);
    assert_eq!(record.gate.outcome, GateOutcome::Deny);
    let EvidencePayload::Rejected { reason } = &record.payload else {
        return Err("expected Rejected payload".to_owned());
    };
    assert_eq!(reason.code, "gate-bot-machine-outside");
    let bot_param = reason
        .params
        .iter()
        .find(|(k, _)| k == "bot")
        .map(|(_, v)| v.as_str());
    let path_param = reason
        .params
        .iter()
        .find(|(k, _)| k == "path")
        .map(|(_, v)| v.as_str());
    assert_eq!(bot_param, Some("Bot1"));
    assert_eq!(path_param, Some("/home/other-bot/x"));
    Ok(())
}

#[tokio::test]
async fn t_sandbox_n4_2() -> Result<(), String> {
    let setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        owner: Some(ComputerOwner::Bot),
        platform: ComputerPlatform::BotMachine,
        computer_name: "bot-computer".to_owned(),
        bot_name: "Bot1".to_owned(),
        workdir: "/home/bot1".to_owned(),
        files: vec![(
            "/home/other-bot/x".to_owned(),
            "OTHER_BOT_SECRET".to_owned(),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "read_file",
        "{\"path\":\"/home/other-bot/x\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "read it").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(&world, &task);
    assert!(
        !kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::ToolStarted { .. })),
        "tool must not start"
    );
    let record = evidence_of_kind(&kinds, EvidenceKind::Rejected).ok_or("missing Rejected")?;
    assert_bot_outside_record(record)?;
    let expected = fmt_text(
        Lang::ZhTw,
        "gate-bot-machine-outside",
        &Params::new()
            .with_text("bot", "Bot1")
            .with_text("path", "/home/other-bot/x"),
    )?;
    let body = request_body(&world.model, 1)?;
    let text = serde_json::to_string(&messages_of(&body)).map_err(|e| format!("{e:?}"))?;
    assert!(text.contains(&expected), "missing expected refusal: {text}");
    assert!(!text.contains("OTHER_BOT_SECRET"));
    Ok(())
}

#[tokio::test]
async fn bot_machine_result_has_no_computer_prefix() -> Result<(), String> {
    let setup = WorldSetup {
        scripts: vec![("ls".to_owned(), CommandScript::ok("a.txt"))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"ls\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "ls").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(&world, &task);
    let summary = completed_summary(&kinds).ok_or("missing ToolCompleted")?;
    assert!(
        !summary.starts_with('['),
        "bot machine summary must not carry [computer] prefix: {summary}"
    );
    Ok(())
}

#[tokio::test]
async fn floor_deny_records_step_two() -> Result<(), String> {
    let world = TurnWorld::start(&user_mac_setup()).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"sudo rm -rf /\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "wipe").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let kinds = task_kinds(&world, &task);
    let gate = first_gate_result(&kinds).ok_or("missing GateResult")?;
    assert_eq!(gate.gate, GateStep::Floor);
    assert_eq!(gate.gate.number(), 2);
    assert_eq!(gate.layer, RuleLayer::Floor);
    assert_eq!(gate.outcome, GateOutcome::Deny);
    Ok(())
}
