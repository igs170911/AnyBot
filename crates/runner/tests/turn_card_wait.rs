use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::time::Duration;

use anybot_protocol::{
    CardAnswer, KnownTaskEventKind, NotRunReason, TaskDetail, TaskStatus, TurnEndReason,
    TurnInitiator,
};
use anybot_runner::{TurnError, TurnOutcome};
use anybot_trust::{ActionKey, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{state_of, task_kinds, wait_for_shown_card};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call};
use test_harness::{advance_clock, model_replies_text, model_tool_calls, sleep_real, timeout_real};

const NPM: &str = "npm install";

fn ask_npm_setup() -> WorldSetup {
    let mut gate = GateConfig::plain();
    gate.user.ask_style.push(ActionKey::command("shell", NPM));
    WorldSetup {
        tools: all_specs(),
        scripts: vec![(NPM.to_owned(), CommandScript::ok("installed"))],
        gate,
        ..WorldSetup::default()
    }
}

fn npm_call(id: &str) -> test_harness::Step {
    model_tool_calls(
        vec![call("shell", &format!("{{\"line\":\"{NPM}\"}}"), id)],
        "tool_calls",
        false,
    )
}

async fn join_turn(
    bg: tokio::task::JoinHandle<Result<TurnOutcome, TurnError>>,
) -> Result<TurnOutcome, String> {
    timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("turn error: {e:?}"))
}

#[tokio::test]
async fn own_card_waits_past_fifteen_minutes() -> Result<(), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(&ask_npm_setup()).await?);
    world.script(vec![npm_call("c1"), model_replies_text("done", false)]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "install").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    let _wakes = advance_clock(&world.clock, Duration::from_mins(20));
    sleep_real(Duration::from_millis(20)).await;
    assert!(!bg.is_finished());

    let kinds = task_kinds(&world, &task);
    assert!(
        !kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::TurnEnded { .. }))
    );
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::AwaitingUser);
    assert!(matches!(state.1, Some(TaskDetail::ApprovalPending { .. })));

    world.answer_shown_card(task, CardAnswer::AllowOnce).await?;
    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    Ok(())
}

#[tokio::test]
async fn unattended_card_times_out_after_fifteen_minutes() -> Result<(), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(&ask_npm_setup()).await?);
    world.script(vec![model_replies_text("first", false)]);
    let initial = world
        .send(Device::Phone, "prepare")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = initial.task;

    world.script(vec![npm_call("c2")]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move {
        shared
            .continue_task(task, TurnInitiator::Verification, None)
            .await
    });

    let (_same, _cards) = wait_for_shown_card(&world, 1).await?;
    let _wakes = advance_clock(&world.clock, Duration::from_secs(899));
    sleep_real(Duration::from_millis(20)).await;
    assert!(!bg.is_finished());
    let mid = state_of(&world, &task)?;
    assert_eq!(mid.0, TaskStatus::AwaitingUser);
    assert!(matches!(mid.1, Some(TaskDetail::ApprovalPending { .. })));

    let _wakes_two = advance_clock(&world.clock, Duration::from_secs(2));
    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::CardTimedOut);
    assert_eq!(outcome.pending.len(), 1);

    let kinds = task_kinds(&world, &task);
    assert!(
        !kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::CardAnswered { .. }))
    );
    assert_eq!(
        state_of(&world, &task)?,
        (TaskStatus::AwaitingUser, Some(TaskDetail::TimedOut))
    );
    Ok(())
}

#[tokio::test]
async fn stop_while_card_waiting_writes_deny_and_skips_action() -> Result<(), String> {
    let mut gate = GateConfig::plain();
    gate.user
        .ask_style
        .push(ActionKey::file("edit_file", "edit", "a.txt"));
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "keep".to_owned())],
        gate,
        ..WorldSetup::default()
    };
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![model_tool_calls(
        vec![edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"keep\",\"new_text\":\"gone\"}",
            "c1",
        )],
        "tool_calls",
        false,
    )]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "change a.txt").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    world.append_task_stopped(task).await?;
    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::Stopped);
    assert_eq!(world.tree_content("a.txt").as_deref(), Some("keep"));

    let kinds = task_kinds(&world, &task);
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::CardAnswered {
            answer: CardAnswer::Deny
        }
    )));
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::ToolNotRun {
            reason: NotRunReason::Stopped,
            ..
        }
    )));
    assert_eq!(
        state_of(&world, &task)?,
        (TaskStatus::Stopped, Some(TaskDetail::YouStopped))
    );
    Ok(())
}
