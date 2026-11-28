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

use anybot_foundation::Clock;
use anybot_protocol::{KnownTaskEventKind, TaskEventKind, TurnEndReason, TurnInitiator};
use anybot_runner::{NewTaskEvent, RecordEntry, StoredBody, TurnError};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup, arc_world, share};
use runner_test::turn_helpers::{messages_of, request_body, task_kinds, task_of};
use runner_test::turn_tool_helpers::call;
use test_harness::{
    advance_clock, arm_model_hold, model_hold_then_finish, model_replies_text, model_tool_calls,
    sleep_real, timeout_real,
};

#[tokio::test]
async fn t_chat_steer_4() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = arc_world(TurnWorld::start(&setup).await?);
    let release = arm_model_hold();
    world.script(vec![
        model_hold_then_finish("\u{597d}", true),
        model_replies_text("ok", false),
    ]);

    let bg = {
        let shared = share(&world);
        tokio::spawn(async move { shared.send(Device::Phone, "A").await })
    };
    wait_for_requests(&world, 1).await?;
    sleep_real(Duration::from_millis(50)).await;

    let queued = world
        .steer(Device::Phone, "B")
        .map_err(|e| format!("steer: {e:?}"))?;
    assert_eq!(queued.turn, 1);

    sleep_real(Duration::from_secs(2)).await;
    let _released = release.send(true);

    let outcome = timeout_real(bg, Duration::from_secs(10))
        .await
        .map_err(|e| format!("timeout: {e}"))?
        .map_err(|e| format!("join: {e}"))?
        .map_err(|e| format!("turn: {e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    assert_eq!(outcome.turn, 2);
    assert!(outcome.unread_steers.is_empty());

    let requests = world.chat_requests();
    assert_eq!(requests.len(), 2);
    let first_users = user_messages_of(world.model.as_ref(), 0)?;
    assert!(!first_users.iter().any(|text| text.contains('B')));
    let second_users = user_messages_of(world.model.as_ref(), 1)?;
    assert!(
        second_users
            .last()
            .is_some_and(|text| text.starts_with('B')),
        "expected last user message to start with B, got {second_users:?}"
    );
    Ok(())
}

fn user_messages_of(model: &test_harness::FakeModel, index: usize) -> Result<Vec<String>, String> {
    let body = request_body(model, index)?;
    Ok(messages_of(&body)
        .iter()
        .filter_map(|m| {
            let role = m.get("role").and_then(|v| v.as_str())?;
            if role != "user" {
                return None;
            }
            let content = m.get("content").and_then(|v| v.as_str())?;
            if content.starts_with("turn-program-note") {
                return None;
            }
            Some(content.to_owned())
        })
        .collect())
}

#[tokio::test]
async fn steer_reaches_next_step() -> Result<(), String> {
    let setup = WorldSetup {
        scripts: vec![("ls".to_owned(), CommandScript::ok("listed"))],
        ..WorldSetup::default()
    };
    let world = arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![
        model_tool_calls(
            vec![call("shell", "{\"command\":\"ls\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);

    let steered = Arc::new(AtomicBool::new(false));
    let shared = share(&world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::ToolStarted { .. })
        ) && !steered.swap(true, Ordering::SeqCst)
        {
            let _queued = shared.steer(Device::Desktop, "B");
        }
    })));

    let outcome = world
        .send(Device::Phone, "A")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    assert_eq!(outcome.turn, 1);

    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let created_count = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::TaskCreated { .. }))
        .count();
    assert_eq!(created_count, 1);

    let requests = world.chat_requests();
    assert_eq!(requests.len(), 2);
    let first_users = user_messages_of(world.model.as_ref(), 0)?;
    let second_users = user_messages_of(world.model.as_ref(), 1)?;
    assert!(!first_users.iter().any(|text| text.contains('B')));
    assert!(second_users.iter().any(|text| text == "B"));
    Ok(())
}

#[tokio::test]
async fn steer_during_verification_round_refused() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = arc_world(TurnWorld::start(&setup).await?);
    assert!(matches!(
        world.steer(Device::Phone, "idle"),
        Err(TurnError::NoOwnTurn)
    ));

    world.script(vec![model_replies_text("first", false)]);
    let first = world
        .send(Device::Phone, "A")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&first);

    let release = arm_model_hold();
    world.script(vec![model_hold_then_finish("verified", true)]);
    let bg = {
        let shared = share(&world);
        tokio::spawn(async move {
            shared
                .continue_task(task, TurnInitiator::Verification, None)
                .await
        })
    };
    wait_for_requests(&world, 2).await?;
    let refused = world.steer(Device::Phone, "B");
    assert!(matches!(refused, Err(TurnError::NoOwnTurn)));

    let _released = release.send(true);
    let outcome = timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("timeout: {e}"))?
        .map_err(|e| format!("join: {e}"))?
        .map_err(|e| format!("continue: {e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    let second_users = user_messages_of(world.model.as_ref(), 1)?;
    assert!(!second_users.iter().any(|text| text.contains('B')));
    Ok(())
}

#[tokio::test]
async fn stop_returns_unread_steers() -> Result<(), String> {
    let setup = WorldSetup {
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        ..WorldSetup::default()
    };
    let world = arc_world(TurnWorld::start(&setup).await?);
    world.script(vec![
        model_tool_calls(
            vec![call("read_file", "{\"path\":\"a.txt\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_replies_text("unreachable", false),
    ]);

    let steered = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let shared = share(&world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { task, event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::ToolStarted { .. })
        ) && !steered.swap(true, Ordering::SeqCst)
        {
            let _queued = shared.steer(Device::Phone, "B");
        }
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::ActionVerified { .. })
        ) && !stopped.swap(true, Ordering::SeqCst)
        {
            advance_clock(&shared.clock, Duration::from_millis(1));
            let _stored = shared.record.append_sync(RecordEntry::Task(NewTaskEvent {
                bot: shared.bot,
                task: *task,
                turn: None,
                event_id: anybot_foundation::EventId::generate(&*shared.clock),
                at_ms: Clock::now_ms(&*shared.clock),
                kind: TaskEventKind::Known(KnownTaskEventKind::TaskStopped),
            }));
        }
    })));

    let outcome = world
        .send(Device::Phone, "A")
        .await
        .map_err(|e| format!("{e:?}"))?;
    world.record.set_after_append(None);
    assert_eq!(outcome.reason, TurnEndReason::Stopped);
    assert_eq!(outcome.unread_steers, vec!["B".to_owned()]);
    for index in 0..world.chat_requests().len() {
        let users = user_messages_of(world.model.as_ref(), index)?;
        assert!(!users.iter().any(|text| text.contains('B')));
    }
    Ok(())
}

async fn wait_for_requests(world: &TurnWorld, target: usize) -> Result<(), String> {
    for _step in 0..400 {
        if world.chat_requests().len() >= target {
            return Ok(());
        }
        sleep_real(Duration::from_millis(5)).await;
    }
    Err(format!("timed out waiting for {target} requests"))
}
