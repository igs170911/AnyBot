use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::sync::Arc;
use std::time::Duration;

use anybot_protocol::{KnownTaskEventKind, TaskDetail, TaskStatus, TurnEndReason};
use anybot_runner::StoredBody;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    chat_requests, drive, kind_names, raw_items_len, record_len, state_of, task_kinds, task_of,
};
use test_harness::{model_accept_then_delay, model_replies_text, model_reply_status, timeout_real};

#[tokio::test]
async fn turn_context_budget_short_sends_nothing() -> Result<(), String> {
    let setup = WorldSetup {
        bot_context: 1_000,
        reserved_output: 200,
        instructions: "x ".repeat(4_000),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("hi", false)]);
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 0);
    assert_eq!(
        kind_names(&world, &task),
        ["task_created", "turn_started", "turn_ended"]
    );
    let kinds = task_kinds(&world, &task);
    let Some(KnownTaskEventKind::TurnEnded { reason, .. }) = kinds.get(2) else {
        return Err("no turn_ended".to_owned());
    };
    let TurnEndReason::ContextBudget {
        deficit_tokens: Some(deficit),
    } = reason
    else {
        return Err("not context budget".to_owned());
    };
    assert!(*deficit > 0);
    assert_eq!(raw_items_len(&world.raw), 0);
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::Stuck);
    assert_eq!(state.1, Some(TaskDetail::ContextBudgetInsufficient));
    Ok(())
}

#[tokio::test]
async fn turn_gateway_refusal_fails_task() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_reply_status(401, "invalid key")]);
    let outcome = world.send(Device::Phone, "go").await;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 1);
    assert_eq!(
        kind_names(&world, &task),
        [
            "task_created",
            "turn_started",
            "context_built",
            "model_asked",
            "turn_ended"
        ]
    );
    let kinds = task_kinds(&world, &task);
    let Some(KnownTaskEventKind::TurnEnded { reason, .. }) = kinds.get(4) else {
        return Err("no turn_ended".to_owned());
    };
    assert_eq!(
        *reason,
        TurnEndReason::ModelFailed {
            code: "model-auth".to_owned()
        }
    );
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::Failed);
    assert_eq!(
        state.1,
        Some(TaskDetail::Error {
            reason: "model-auth".to_owned()
        })
    );
    Ok(())
}

#[tokio::test]
async fn turn_blank_reply_fails_task() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_replies_text("", false),
        model_replies_text("", false),
    ]);
    let outcome = Box::pin(drive(&world.clock, world.send(Device::Phone, "go"))).await?;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 2);
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::Failed);
    assert_eq!(
        state.1,
        Some(TaskDetail::Error {
            reason: "model-blank".to_owned()
        })
    );
    Ok(())
}

#[tokio::test]
async fn turn_record_failure_stops_before_request() -> Result<(), String> {
    let setup = WorldSetup {
        fail_on_kind: Some("model_asked"),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("hi", false)]);
    let first = world.send(Device::Phone, "go").await;
    assert!(first.is_err());
    assert_eq!(chat_requests(&world.model).len(), 0);
    let task = created_task(&world).ok_or("no created task")?;
    assert_eq!(
        kind_names(&world, &task),
        ["task_created", "turn_started", "context_built"]
    );
    let second = world.send(Device::Phone, "again").await;
    let text = format!("{:?}", second.err());
    assert!(text.contains("Record"));
    Ok(())
}

fn created_task(world: &TurnWorld) -> Option<anybot_foundation::TaskId> {
    for row in world.record.snapshot() {
        let StoredBody::Task { task, event, .. } = &row.body else {
            continue;
        };
        if event.seq == 1 {
            return Some(*task);
        }
    }
    None
}

#[tokio::test]
async fn turn_second_message_while_running_is_busy() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = Arc::new(TurnWorld::start(&setup).await?);
    world.script(vec![model_accept_then_delay(
        5,
        model_replies_text("first", false),
    )]);
    world.model.header_clock(Arc::clone(&world.clock));
    let shared = runner_test::turn::share(&world);
    let first = tokio::spawn(run_shared(shared, Device::Phone, "first"));
    wait_for_request(&world, 1).await?;
    let len_before = record_len(&world.record);
    let second = world.send(Device::Phone, "second").await;
    assert!(second.is_err());
    assert_eq!(record_len(&world.record), len_before);
    let joined = async move { first.await.map_err(|e| format!("{e:?}")) };
    let outcome = drive(&world.clock, joined).await?;
    let outcome = outcome.map_err(|_e| "join failed".to_owned())?;
    let outcome = outcome.map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert!(kind_names(&world, &task).contains(&"task_created".to_owned()));
    assert_eq!(count_created(&world), 1);
    let kinds = task_kinds(&world, &task);
    let Some(KnownTaskEventKind::TurnEnded { reason, .. }) = kinds.last() else {
        return Err("no turn_ended".to_owned());
    };
    assert_eq!(*reason, TurnEndReason::Answered);
    Ok(())
}

fn count_created(world: &TurnWorld) -> usize {
    let mut count = 0usize;
    for row in world.record.snapshot() {
        let StoredBody::Task { event, .. } = &row.body else {
            continue;
        };
        if event.kind.kind_name() == "task_created" {
            count = count.checked_add(1).unwrap_or(count);
        }
    }
    count
}

async fn run_shared(
    world: Arc<TurnWorld>,
    from: Device,
    text: &str,
) -> Result<anybot_runner::TurnOutcome, anybot_runner::TurnError> {
    world.send(from, text).await
}

async fn wait_for_request(world: &Arc<TurnWorld>, count: usize) -> Result<(), String> {
    let arrived = world.model.wait_recorded(count);
    timeout_real(arrived, Duration::from_secs(10))
        .await
        .map_err(|e| format!("model request {count} never arrived: {e}"))?
}

use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use sha2 as _;
