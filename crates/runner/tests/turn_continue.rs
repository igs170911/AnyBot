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

use std::time::Duration;

use anybot_protocol::{KnownTaskEventKind, TaskDetail, TaskStatus, TurnEndReason, TurnInitiator};
use anybot_runner::{StoredBody, TurnError};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{state_of, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, edit_call};
use test_harness::{
    arm_model_hold, model_hold_then_finish, model_replies_text, model_tool_calls, sleep_real,
    timeout_real,
};

#[tokio::test]
async fn continue_task_reuses_task_and_numbers() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"one\"}",
                "c1",
            )],
            "tool_calls",
            false,
        ),
        model_replies_text("done a", false),
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"b.txt\",\"old_text\":\"\",\"new_text\":\"two\"}",
                "c2",
            )],
            "tool_calls",
            false,
        ),
        model_replies_text("done b", false),
        model_replies_text("verified", false),
    ]);
    let first = world
        .send(Device::Phone, "make a.txt")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&first);
    assert_eq!(first.turn, 1);

    let second = world
        .continue_from(task, Device::Desktop, Some("make b.txt"))
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(second.task, task);
    assert_eq!(second.turn, 2);
    assert_eq!(second.reason, TurnEndReason::Answered);
    assert_eq!(world.tree_content("a.txt").as_deref(), Some("one"));
    assert_eq!(world.tree_content("b.txt").as_deref(), Some("two"));

    let third = world
        .continue_task(task, TurnInitiator::Verification, None)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(third.task, task);
    assert_eq!(third.turn, 3);
    assert_eq!(third.reason, TurnEndReason::Answered);

    check_numbers_and_state(&world, task)
}

fn check_numbers_and_state(
    world: &TurnWorld,
    task: anybot_foundation::TaskId,
) -> Result<(), String> {
    let kinds = task_kinds(world, &task);
    let created = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::TaskCreated { .. }))
        .count();
    assert_eq!(created, 1);

    let turns: Vec<(u32, TurnInitiator)> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::TurnStarted {
                turn, initiator, ..
            } = k
            else {
                return None;
            };
            Some((*turn, *initiator))
        })
        .collect();
    assert_eq!(
        turns,
        vec![
            (
                1,
                TurnInitiator::You {
                    device: world.device(Device::Phone),
                },
            ),
            (
                2,
                TurnInitiator::You {
                    device: world.device(Device::Desktop),
                },
            ),
            (3, TurnInitiator::Verification),
        ]
    );
    check_evidence_and_steps(&kinds);
    check_stored_turns(world, task);
    assert_eq!(
        state_of(world, &task)?,
        (TaskStatus::Completed, Some(TaskDetail::Accepted))
    );
    Ok(())
}

fn check_evidence_and_steps(kinds: &[KnownTaskEventKind]) {
    let shorts: Vec<u32> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::EvidenceAdded { record } = k else {
                return None;
            };
            Some(record.short.0)
        })
        .collect();
    assert_eq!(shorts, vec![1, 2]);

    let steps: Vec<u32> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ModelAsked { step } = k else {
                return None;
            };
            Some(*step)
        })
        .collect();
    assert_eq!(steps, vec![1, 2, 3, 4, 5]);
}

fn check_stored_turns(world: &TurnWorld, task: anybot_foundation::TaskId) {
    let stored_turns: Vec<Option<u32>> = world
        .record
        .snapshot()
        .into_iter()
        .filter_map(|row| match row.body {
            StoredBody::Task {
                task: owner, turn, ..
            } if owner == task => Some(turn),
            StoredBody::Task { .. } | StoredBody::UserSaid(_) => None,
        })
        .collect();
    assert!(stored_turns.contains(&Some(1)));
    assert!(stored_turns.contains(&Some(2)));
    assert!(stored_turns.contains(&Some(3)));
}

#[tokio::test]
async fn continue_cancelled_task_is_refused() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("first", false)]);
    let first = world
        .send(Device::Phone, "hello")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&first);
    world.append_task_cancelled(task).await?;
    let before = world.record.len();

    let res = world.continue_from(task, Device::Phone, Some("more")).await;
    assert!(matches!(res, Err(TurnError::TaskClosed { task: got }) if got == task));
    assert_eq!(world.record.len(), before);

    let missing = anybot_foundation::TaskId::generate(&*world.clock);
    let missing_res = world
        .continue_from(missing, Device::Phone, Some("hi"))
        .await;
    assert!(matches!(
        missing_res,
        Err(TurnError::TaskNotFound { task: got }) if got == missing
    ));
    assert_eq!(world.record.len(), before);
    Ok(())
}

#[tokio::test]
async fn continue_while_busy_is_refused() -> Result<(), String> {
    let setup = WorldSetup::default();
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    let release = arm_model_hold();
    world.script(vec![model_hold_then_finish("ok", false)]);
    let existing = anybot_foundation::TaskId::generate(&*world.clock);

    let bg = {
        let shared = runner_test::turn::share(&world);
        tokio::spawn(async move { shared.send(Device::Phone, "hold").await })
    };
    for _step in 0..200 {
        if !world.chat_requests().is_empty() {
            break;
        }
        sleep_real(Duration::from_millis(5)).await;
    }

    let busy = world
        .continue_from(existing, Device::Phone, Some("interrupt"))
        .await;
    assert!(matches!(busy, Err(TurnError::Busy)));

    let _sent = release.send(true);
    let first = timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("join timed out: {e}"))?
        .map_err(|e| format!("join error: {e}"))?
        .map_err(|e| format!("send failed: {e:?}"))?;
    assert_eq!(first.reason, TurnEndReason::Answered);
    Ok(())
}
