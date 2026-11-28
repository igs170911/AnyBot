use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_tools as _;
use anybot_trust as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;
use tokio as _;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use anybot_foundation::{Clock, TaskId};
use anybot_protocol::{KnownTaskEventKind, TaskEventKind, TurnEndReason};
use anybot_runner::{NewTaskEvent, RecordEntry, StoredBody};
use proptest::prelude::*;
use proptest::test_runner::{Config as PropConfig, TestRunner};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::task_of;
use runner_test::turn_tool_helpers::all_specs;
use test_harness::{Step, ToolCallScript, advance_clock, model_replies_text, model_tool_calls};

fn stop_event(world: &TurnWorld, task: TaskId) -> RecordEntry {
    RecordEntry::Task(NewTaskEvent {
        bot: world.bot,
        task,
        turn: None,
        event_id: anybot_foundation::EventId::generate(&*world.clock),
        at_ms: world.clock.as_ref_now_ms(),
        kind: TaskEventKind::Known(KnownTaskEventKind::TaskStopped),
    })
}

trait ClockExt {
    fn as_ref_now_ms(&self) -> anybot_foundation::TimestampMs;
}

impl ClockExt for test_harness::FakeClock {
    fn as_ref_now_ms(&self) -> anybot_foundation::TimestampMs {
        Clock::now_ms(self)
    }
}

fn read_step(id: &str) -> Step {
    model_tool_calls(
        vec![ToolCallScript {
            name: "read_file".to_owned(),
            arguments: "{\"path\":\"a.txt\"}".to_owned(),
            id: Some(id.to_owned()),
            argument_parts: Vec::new(),
        }],
        "tool_calls",
        false,
    )
}

fn five_step_script() -> Vec<Step> {
    vec![
        read_step("s1"),
        read_step("s2"),
        read_step("s3"),
        read_step("s4"),
        read_step("s5"),
        model_replies_text("done", false),
    ]
}

#[test]
fn t_chat_stop_9() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        ..WorldSetup::default()
    };
    let world = Arc::new(runtime.block_on(TurnWorld::start(&setup))?);
    let mut runner = TestRunner::new(PropConfig::with_cases(1_000));
    runner
        .run(&(0usize..38usize), |stop_at| {
            let res = run_stop_prop_case(&runtime, &world, stop_at);
            prop_assert_eq!(res, Ok(()));
            Ok(())
        })
        .map_err(|e| format!("proptest failed: {e}"))
}

fn run_stop_prop_case(
    runtime: &tokio::runtime::Runtime,
    world: &Arc<TurnWorld>,
    stop_at: usize,
) -> Result<(), String> {
    world.record.clear();
    world.script(five_step_script());
    install_stop_hook(world, stop_at);
    let outcome = runtime
        .block_on(world.send(Device::Phone, "run five"))
        .map_err(|e| format!("send: {e:?}"))?;
    world.record.set_after_append(None);
    if outcome.reason != TurnEndReason::Stopped {
        return Err(format!("expected Stopped got {:?}", outcome.reason));
    }
    verify_all_started_before_stop(world, task_of(&outcome))
}

fn install_stop_hook(world: &Arc<TurnWorld>, stop_at: usize) {
    let seen = Arc::new(AtomicUsize::new(0));
    let turn_started = Arc::new(AtomicBool::new(false));
    let stopped = Arc::new(AtomicBool::new(false));
    let shared = Arc::clone(world);
    world.record.set_after_append(Some(Arc::new(move |row| {
        let StoredBody::Task { task, event, .. } = &row.body else {
            return;
        };
        if matches!(
            event.kind,
            TaskEventKind::Known(KnownTaskEventKind::TurnStarted { .. })
        ) {
            turn_started.store(true, Ordering::SeqCst);
        }
        if !turn_started.load(Ordering::SeqCst) {
            return;
        }
        let n = seen.fetch_add(1, Ordering::SeqCst);
        if n >= stop_at && !stopped.swap(true, Ordering::SeqCst) {
            advance_clock(&shared.clock, Duration::from_millis(1));
            let _stored = shared.record.append_sync(stop_event(&shared, *task));
        }
    })));
}

fn verify_all_started_before_stop(world: &TurnWorld, task: TaskId) -> Result<(), String> {
    let events = world.task_events(task);
    let stopped = events
        .iter()
        .find(|e| {
            matches!(
                e.kind,
                TaskEventKind::Known(KnownTaskEventKind::TaskStopped)
            )
        })
        .ok_or_else(|| "missing TaskStopped".to_owned())?;
    for event in &events {
        let TaskEventKind::Known(kind) = &event.kind else {
            continue;
        };
        if matches!(
            kind,
            KnownTaskEventKind::ModelAsked { .. } | KnownTaskEventKind::ToolStarted { .. }
        ) {
            if event.seq >= stopped.seq {
                return Err(format!("seq {} >= stop {}", event.seq, stopped.seq));
            }
            if event.at_ms >= stopped.at_ms {
                return Err(format!("ms {:?} >= stop {:?}", event.at_ms, stopped.at_ms));
            }
        }
    }
    Ok(())
}
