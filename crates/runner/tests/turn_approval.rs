use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use std::borrow::ToOwned;

use anybot_protocol::{CardAnswer, EvidenceKind, KnownTaskEventKind, TaskDetail, TaskStatus};
use anybot_trust::{ActionKey, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    answer_card, card_of, request_body, shown_card, state_of, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{all_specs, one_tool_step};
use test_harness::advance_clock;

const NPM: &str = "npm install";
const REFUSED_CODE: &str = "gate-ask-declined";

fn ask_setup() -> WorldSetup {
    let mut gate = GateConfig::plain();
    gate.user.ask_style.push(ActionKey::command("shell", NPM));
    WorldSetup {
        tools: all_specs(),
        scripts: vec![("npm install".to_owned(), CommandScript::ok("installed"))],
        gate,
        ..WorldSetup::default()
    }
}

fn npm_call_step(id: &str) -> Vec<test_harness::Step> {
    one_tool_step(vec![runner_test::turn_tool_helpers::call(
        "shell",
        &format!("{{\"line\":\"{NPM}\"}}"),
        id,
    )])
}

fn has_card(kinds: &[KnownTaskEventKind], digest: &str) -> bool {
    kinds
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::CardShown { card, .. } if card == digest))
}

#[tokio::test]
async fn t_approval_answers_1() -> Result<(), String> {
    let setup = ask_setup();
    let world = TurnWorld::start(&setup).await?;
    world.script(npm_call_step("c1"));
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(
        outcome.reason,
        anybot_protocol::TurnEndReason::PendingApproval
    );
    assert_eq!(outcome.pending.len(), 1);
    let kinds = task_kinds(&world, &task);
    let digest = shown_card(&outcome, &kinds).ok_or_else(|| "no shown card".to_owned())?;
    assert_eq!(
        card_of(&outcome, &digest).map(|c| c.tool.clone()),
        Some("shell".to_owned())
    );
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::AwaitingUser);
    assert!(matches!(state.1, Some(TaskDetail::ApprovalPending { .. })));
    answer_card(&world, &outcome, &digest, CardAnswer::AllowOnce).await?;
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(npm_call_step("c2"));
    let second = world.send(Device::Phone, "again").await;
    let second = second.map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        second.reason,
        anybot_protocol::TurnEndReason::PendingApproval
    );
    let second_task = task_of(&second);
    let second_kinds = task_kinds(&world, &second_task);
    let second_digest =
        shown_card(&second, &second_kinds).ok_or_else(|| "second card".to_owned())?;
    assert_ne!(second_digest, digest);
    assert!(has_card(&second_kinds, &second_digest));
    Ok(())
}

#[tokio::test]
async fn declined_card_refusal_reaches_next_request() -> Result<(), String> {
    let setup = ask_setup();
    let world = TurnWorld::start(&setup).await?;
    world.script(npm_call_step("c1"));
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let digest = shown_card(&outcome, &kinds).ok_or_else(|| "no shown card".to_owned())?;
    answer_card(&world, &outcome, &digest, CardAnswer::Deny).await?;
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(npm_call_step("c2"));
    let again = world.send(Device::Phone, "again").await;
    let _again = again.map_err(|e| format!("{e:?}"))?;
    let refusal = REFUSED_CODE;
    world.script(npm_call_step("c3"));
    let _after = world.send(Device::Phone, "post").await;
    let bodies = world.model.recorded().len();
    let mut found = false;
    for index in 0..bodies {
        let body = request_body(world.model.as_ref(), index)?;
        let text = serde_json::to_string(&body).map_err(|e| format!("{e}"))?;
        if text.contains(refusal) {
            found = true;
        }
    }
    assert!(!found);
    let kinds = task_kinds(&world, &task);
    let has_refusal = kinds.iter().any(|k| {
        matches!(k, KnownTaskEventKind::ToolCompleted { summary, .. } if summary.contains(refusal))
    });
    assert!(has_refusal);
    let after = state_of(&world, &task)?;
    assert_eq!(after.0, TaskStatus::Running);
    Ok(())
}

#[tokio::test]
async fn stop_answered_card_parks_again() -> Result<(), String> {
    let setup = ask_setup();
    let world = TurnWorld::start(&setup).await?;
    world.script(npm_call_step("c1"));
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let digest = shown_card(&outcome, &kinds).ok_or_else(|| "no shown card".to_owned())?;
    answer_card(&world, &outcome, &digest, CardAnswer::Deny).await?;
    world.append_task_stopped(task).await?;
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(npm_call_step("c2"));
    let retry = world.send(Device::Phone, "retry").await;
    let retry = retry.map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        retry.reason,
        anybot_protocol::TurnEndReason::PendingApproval
    );
    assert_eq!(retry.pending.len(), 1);
    Ok(())
}

#[tokio::test]
async fn allow_conversation_executes_next_calls() -> Result<(), String> {
    let setup = runner_test::trust_world::mac_setup("Work Mac");
    let world = TurnWorld::start(&setup).await?;
    let make_step = |id: &str| {
        one_tool_step(vec![runner_test::turn_tool_helpers::call(
            "shell",
            "{\"line\":\"make test\"}",
            id,
        )])
    };
    world.script(make_step("c1"));
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let digest = shown_card(&outcome, &kinds).ok_or_else(|| "no shown card".to_owned())?;
    answer_card(&world, &outcome, &digest, CardAnswer::AllowConversation).await?;
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(make_step("c2"));
    let second = world.send(Device::Phone, "again").await;
    let second = second.map_err(|e| format!("{e:?}"))?;
    assert_eq!(second.reason, anybot_protocol::TurnEndReason::Answered);
    let second_kinds = task_kinds(&world, &task_of(&second));
    let has_executed = second_kinds
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed));
    assert!(has_executed);
    Ok(())
}
