use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{EvidencePayload, KnownTaskEventKind, ReasonPayload};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{kind_names, task_kinds, task_of};
use test_harness::{ToolCallScript, model_replies_text, model_tool_calls};

fn call(name: &str, args: &str, id: &str) -> ToolCallScript {
    ToolCallScript {
        name: name.to_owned(),
        arguments: args.to_owned(),
        id: Some(id.to_owned()),
        argument_parts: Vec::new(),
    }
}

#[tokio::test]
async fn t_turn_loop_3() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![
                call(
                    "edit_file",
                    "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
                    "c1",
                ),
                call("shell", "{\"command\":\"sudo rm -rf /\"}", "c2"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("ok", false),
    ]);
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let events = task_events(&world, &task);
    check_prefix_order(&events)?;
    check_first_action_order(&world, &task)?;
    check_second_action_denied(&world, &task)?;
    check_seq_contiguous(&events)?;
    Ok(())
}

fn task_events(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Vec<anybot_protocol::TaskEvent> {
    world.record.task_events_of(*task)
}

fn check_prefix_order(events: &[anybot_protocol::TaskEvent]) -> Result<(), String> {
    let names: Vec<String> = events.iter().map(|e| e.kind.kind_name()).collect();
    let wanted: [&str; 4] = [
        "context_built",
        "model_asked",
        "model_answer",
        "model_usage",
    ];
    let start = names
        .iter()
        .position(|n| n == "context_built")
        .ok_or("no context_built".to_owned())?;
    let head: Vec<String> = names.iter().skip(start).take(4).cloned().collect();
    let wanted_vec: Vec<String> = wanted.iter().map(|s| (*s).to_owned()).collect();
    if head != wanted_vec {
        return Err(format!("prefix {head:?} wanted {wanted_vec:?}"));
    }
    Ok(())
}

fn check_first_action_order(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<(), String> {
    let names = kind_names(world, task);
    let wanted: [&str; 6] = [
        "gate_result",
        "tool_intent",
        "tool_started",
        "tool_completed",
        "evidence_added",
        "action_verified",
    ];
    let wanted_vec: Vec<String> = wanted.iter().map(|s| (*s).to_owned()).collect();
    let start = names
        .iter()
        .position(|n| n == "gate_result")
        .ok_or("no gate_result".to_owned())?;
    let rest = names.len().saturating_sub(start);
    if rest < wanted_vec.len().saturating_add(2) {
        return Err(format!("too few names after gate: {names:?}"));
    }
    let slice: Vec<String> = names
        .iter()
        .skip(start)
        .take(wanted_vec.len())
        .cloned()
        .collect();
    if slice != wanted_vec {
        return Err(format!("first action {slice:?} wanted {wanted_vec:?}"));
    }
    Ok(())
}

fn check_second_action_denied(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<(), String> {
    let names = kind_names(world, task);
    let gates: Vec<usize> = names
        .iter()
        .enumerate()
        .filter(|(_, n)| *n == "gate_result")
        .map(|(i, _)| i)
        .collect();
    if gates.len() != 2 {
        return Err(format!("gate count {}", gates.len()));
    }
    let second = gates.get(1).copied().ok_or("no second gate".to_owned())?;
    let tail: Vec<String> = names.iter().skip(second).take(2).cloned().collect();
    if tail.first().map(String::as_str) != Some("gate_result")
        || tail.get(1).map(String::as_str) != Some("evidence_added")
    {
        return Err(format!("after second gate {tail:?}"));
    }
    for name in names.iter().skip(second.saturating_add(1)) {
        if name == "tool_intent" || name == "tool_started" {
            return Err(format!("denied action has {name}"));
        }
    }
    check_rejected_evidence(world, task)?;
    Ok(())
}

fn check_rejected_evidence(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<(), String> {
    for kind in task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        let EvidencePayload::Rejected { reason } = &record.payload else {
            continue;
        };
        return check_rule_word(reason);
    }
    Err("no rejected evidence".to_owned())
}

fn check_rule_word(reason: &ReasonPayload) -> Result<(), String> {
    for (key, value) in &reason.params {
        if (key == "rule" || key == "token") && !value.is_empty() {
            return Ok(());
        }
    }
    Err(format!("no rule in {:?}", reason.params))
}

fn check_seq_contiguous(events: &[anybot_protocol::TaskEvent]) -> Result<(), String> {
    let mut last = 0u64;
    for event in events {
        if event.seq <= last {
            return Err(format!("seq {} after {}", event.seq, last));
        }
        last = event.seq;
    }
    Ok(())
}
