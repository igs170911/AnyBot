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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anybot_foundation::{Clock, TaskId};
use anybot_protocol::{KnownTaskEventKind, NotRunReason, TaskEventKind, TaskStatus, TurnEndReason};
use anybot_runner::{CODE_TURN_REPLY_STOPPED, NewTaskEvent, RecordEntry, StoredBody};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{state_of, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call};
use test_harness::{advance_clock, model_replies_text, model_tool_calls};

fn stop_event(world: &TurnWorld, task: TaskId) -> RecordEntry {
    RecordEntry::Task(NewTaskEvent {
        bot: world.bot,
        task,
        turn: None,
        event_id: anybot_foundation::EventId::generate(&*world.clock),
        at_ms: Clock::now_ms(&*world.clock),
        kind: TaskEventKind::Known(KnownTaskEventKind::TaskStopped),
    })
}

fn cancel_event(world: &TurnWorld, task: TaskId) -> RecordEntry {
    RecordEntry::Task(NewTaskEvent {
        bot: world.bot,
        task,
        turn: None,
        event_id: anybot_foundation::EventId::generate(&*world.clock),
        at_ms: Clock::now_ms(&*world.clock),
        kind: TaskEventKind::Known(KnownTaskEventKind::TaskCancelled {
            reason: "user".to_owned(),
        }),
    })
}

#[tokio::test]
async fn stop_after_first_action_marks_rest_not_run() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        ..WorldSetup::default()
    };
    let world = Arc::new(TurnWorld::start(&setup).await?);
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("read_file", "{\"path\":\"a.txt\"}", "c2"),
                call("read_file", "{\"path\":\"a.txt\"}", "c3"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("unreachable", false),
    ]);
    install_stop_on_verified(&world);
    let outcome = world
        .send(Device::Phone, "read 3")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(outcome.reason, TurnEndReason::Stopped);
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    assert_eq!(count_kind(&kinds, "model_asked"), 1);
    assert_eq!(count_kind(&kinds, "tool_completed"), 1);
    let not_run: Vec<NotRunReason> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ToolNotRun { reason, .. } = k else {
                return None;
            };
            Some(*reason)
        })
        .collect();
    assert_eq!(not_run, vec![NotRunReason::Stopped, NotRunReason::Stopped]);
    assert_eq!(state_of(&world, &task)?.0, TaskStatus::Stopped);
    Ok(())
}

fn install_stop_on_verified(world: &Arc<TurnWorld>) {
    let stopped = Arc::new(AtomicBool::new(false));
    let shared = Arc::clone(world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { task, event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::ActionVerified { .. })
        ) && !stopped.swap(true, Ordering::SeqCst)
        {
            advance_clock(&shared.clock, Duration::from_millis(1));
            let _stored = shared.record.append_sync(stop_event(&shared, *task));
        }
    })));
}

#[tokio::test]
async fn cancel_ends_turn_cancelled() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        ..WorldSetup::default()
    };
    let world = Arc::new(TurnWorld::start(&setup).await?);
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("read_file", "{\"path\":\"a.txt\"}", "c2"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("unreachable", false),
    ]);
    install_cancel_on_verified(&world);
    let outcome = world
        .send(Device::Phone, "read 2")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(outcome.reason, TurnEndReason::Cancelled);
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    assert_eq!(count_kind(&kinds, "model_asked"), 1);
    assert_eq!(count_kind(&kinds, "tool_completed"), 1);
    assert_eq!(count_kind(&kinds, "tool_not_run"), 1);
    assert_eq!(state_of(&world, &task)?.0, TaskStatus::Cancelled);
    Ok(())
}

fn install_cancel_on_verified(world: &Arc<TurnWorld>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let shared = Arc::clone(world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { task, event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::ActionVerified { .. })
        ) && !cancelled.swap(true, Ordering::SeqCst)
        {
            advance_clock(&shared.clock, Duration::from_millis(1));
            let _stored = shared.record.append_sync(cancel_event(&shared, *task));
        }
    })));
}

#[tokio::test]
async fn stop_without_text_writes_stopped_reply() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        ..WorldSetup::default()
    };
    let world = Arc::new(TurnWorld::start(&setup).await?);
    world.script(vec![model_replies_text("never asked", false)]);
    install_stop_on_turn_started(&world);
    let outcome = world
        .send(Device::Phone, "hi")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(outcome.reason, TurnEndReason::Stopped);
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    assert_eq!(count_kind(&kinds, "context_built"), 0);
    assert_eq!(count_kind(&kinds, "model_asked"), 0);
    assert_eq!(
        answer_texts(&kinds),
        vec![CODE_TURN_REPLY_STOPPED.to_owned()]
    );
    assert_eq!(state_of(&world, &task)?.0, TaskStatus::Stopped);

    world.script(vec![
        model_tool_calls(
            vec![call("read_file", "{\"path\":\"a.txt\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_replies_text("never reached", false),
    ]);
    install_stop_on_verified(&world);
    let second = world
        .send(Device::Phone, "read then stop")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(second.reason, TurnEndReason::Stopped);
    let second_kinds = task_kinds(&world, &task_of(&second));
    assert_eq!(
        answer_texts(&second_kinds).last().map(String::as_str),
        Some(CODE_TURN_REPLY_STOPPED)
    );
    Ok(())
}

fn install_stop_on_turn_started(world: &Arc<TurnWorld>) {
    let stopped = Arc::new(AtomicBool::new(false));
    let shared = Arc::clone(world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { task, event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnStarted { .. })
        ) && !stopped.swap(true, Ordering::SeqCst)
        {
            advance_clock(&shared.clock, Duration::from_millis(1));
            let _stored = shared.record.append_sync(stop_event(&shared, *task));
        }
    })));
}

fn answer_texts(kinds: &[KnownTaskEventKind]) -> Vec<String> {
    kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ModelAnswer { text, .. } = k else {
                return None;
            };
            Some(text.clone())
        })
        .collect()
}

fn count_kind(kinds: &[KnownTaskEventKind], name: &str) -> usize {
    kinds.iter().filter(|k| k.kind_name() == name).count()
}
