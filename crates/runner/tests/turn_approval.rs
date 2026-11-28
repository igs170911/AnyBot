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
    CardAnswer, EvidenceKind, KnownTaskEventKind, NotRunReason, TaskDetail, TaskStatus,
    TurnEndReason,
};
use anybot_trust::{ActionKey, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{request_body, state_of, task_kinds, wait_for_shown_card};
use runner_test::turn_tool_helpers::call;
use test_harness::{model_replies_text, model_tool_calls, timeout_real};

const NPM: &str = "npm install";
const REFUSED_ENGLISH: &str =
    "The user refused this operation. Do not try the same operation again.";

fn ask_setup() -> WorldSetup {
    let mut gate = GateConfig::plain();
    gate.user.ask_style.push(ActionKey::command("shell", NPM));
    WorldSetup {
        scripts: vec![("npm install".to_owned(), CommandScript::ok("installed"))],
        gate,
        ..WorldSetup::default()
    }
}

fn npm_call(id: &str) -> test_harness::Step {
    model_tool_calls(
        vec![call("shell", &format!("{{\"command\":\"{NPM}\"}}"), id)],
        "tool_calls",
        false,
    )
}

async fn join_turn(
    bg: tokio::task::JoinHandle<Result<anybot_runner::TurnOutcome, anybot_runner::TurnError>>,
) -> Result<anybot_runner::TurnOutcome, String> {
    timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("turn error: {e:?}"))
}

#[tokio::test]
async fn t_approval_answers_1() -> Result<(), String> {
    let setup = ask_setup();
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![
        npm_call("c1"),
        npm_call("c2"),
        model_replies_text("done", false),
    ]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "go").await });

    let (task, first_cards) = wait_for_shown_card(&world, 1).await?;
    let first_card = first_cards.first().cloned().ok_or("no first card")?;
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::AwaitingUser);
    assert!(matches!(state.1, Some(TaskDetail::ApprovalPending { .. })));
    world.answer_shown_card(task, CardAnswer::AllowOnce).await?;

    let (_same_task, second_cards) = wait_for_shown_card(&world, 2).await?;
    let second_card = second_cards.get(1).cloned().ok_or("no second card")?;
    assert_ne!(second_card, first_card);
    world.answer_shown_card(task, CardAnswer::AllowOnce).await?;

    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.task, task);
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    assert!(outcome.pending.is_empty());
    Ok(())
}

#[tokio::test]
async fn declined_card_refusal_reaches_next_request() -> Result<(), String> {
    let setup = ask_setup();
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![npm_call("c1"), model_replies_text("ok", false)]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "go").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    world.answer_shown_card(task, CardAnswer::Deny).await?;

    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    let body = request_body(world.model.as_ref(), 1)?;
    let text = serde_json::to_string(&body).map_err(|e| format!("{e}"))?;
    assert!(text.contains(REFUSED_ENGLISH));

    let kinds = task_kinds(&world, &task);
    let has_refusal = kinds.iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::ToolCompleted { summary, .. } if summary.contains(REFUSED_ENGLISH)
        )
    });
    assert!(has_refusal);
    assert_eq!(
        state_of(&world, &task)?,
        (TaskStatus::Unaccepted, Some(TaskDetail::StepRejected))
    );
    Ok(())
}

#[tokio::test]
async fn stop_answered_card_parks_again() -> Result<(), String> {
    let setup = ask_setup();
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![npm_call("c1")]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "go").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    world.append_task_stopped(task).await?;
    let stopped = join_turn(bg).await?;
    assert_eq!(stopped.reason, TurnEndReason::Stopped);

    let first_kinds = task_kinds(&world, &task);
    assert!(first_kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::CardAnswered {
            answer: CardAnswer::Deny
        }
    )));
    assert!(first_kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::ToolNotRun {
            reason: NotRunReason::Stopped,
            ..
        }
    )));

    world.script(vec![npm_call("c2"), model_replies_text("done", false)]);
    let shared_retry = runner_test::turn::share(&world);
    let retry_bg = tokio::spawn(async move {
        shared_retry
            .continue_from(task, Device::Desktop, Some("retry"))
            .await
    });
    let (_same, cards) = wait_for_shown_card(&world, 2).await?;
    assert_eq!(cards.len(), 2);
    world.answer_shown_card(task, CardAnswer::AllowOnce).await?;

    let retried = join_turn(retry_bg).await?;
    assert_eq!(retried.reason, TurnEndReason::Answered);
    Ok(())
}

#[tokio::test]
async fn allow_conversation_executes_next_calls() -> Result<(), String> {
    let setup = ask_setup();
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![
        npm_call("c1"),
        npm_call("c2"),
        model_replies_text("done", false),
    ]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "go").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    allow_in_session(&world);
    world
        .answer_shown_card(task, CardAnswer::AllowConversation)
        .await?;

    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    let kinds = task_kinds(&world, &task);
    let shown_count = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardShown { .. }))
        .count();
    let executed_count = kinds
        .iter()
        .filter(|k| {
            matches!(
                k,
                KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed
            )
        })
        .count();
    assert_eq!(shown_count, 1);
    assert_eq!(executed_count, 2);
    Ok(())
}

fn allow_in_session(world: &TurnWorld) {
    let mut gate = GateConfig::plain();
    gate.user.session.push(ActionKey::command("shell", NPM));
    world.set_gate(gate);
}
