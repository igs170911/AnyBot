use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_domain::TaskView;
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind};
use proptest::prelude::*;
use runner_test::turn::{Device, TurnWorld};
use runner_test::turn_tool_helpers::{call, world_with};
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

const SLOTS: [(&str, &str); 8] = [
    ("shell", "{\"line\":\"ls\"}"),
    ("shell", "{\"line\":\"exit 3\"}"),
    ("shell", "{\"line\":\"sudo rm -rf /\"}"),
    ("read_file", "{\"path\":\"a.txt\"}"),
    ("read_file", "{\"path\":\"missing.txt\"}"),
    (
        "edit_file",
        "{\"path\":\"a.txt\",\"old_text\":\"x\",\"new_text\":\"y\"}",
    ),
    (
        "edit_file",
        "{\"path\":\"missing.txt\",\"old_text\":\"q\",\"new_text\":\"r\"}",
    ),
    (
        "edit_file",
        "{\"path\":\"n.txt\",\"old_text\":\"\",\"new_text\":\"hello\"}",
    ),
];

fn arb_steps() -> BoxedStrategy<Vec<Vec<usize>>> {
    prop::collection::vec(
        prop::collection::vec(0usize..SLOTS.len(), 0usize..4usize),
        1usize..3usize,
    )
    .boxed()
}

fn script_of(steps: &[Vec<usize>]) -> Vec<Step> {
    let mut out: Vec<Step> = Vec::new();
    let mut counter = 0usize;
    for step in steps {
        let mut calls: Vec<ToolCallScript> = Vec::new();
        for slot in step {
            let (name, args) = SLOTS.get(*slot).copied().unwrap_or(("read_file", "{}"));
            counter = counter.saturating_add(1);
            calls.push(call(name, args, &format!("p{counter}")));
        }
        if calls.is_empty() {
            out.push(model_replies_text("done", false));
            return out;
        }
        out.push(model_tool_calls(calls, "tool_calls", false));
    }
    out.push(model_replies_text("done", false));
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_turn_loop_1(steps in arb_steps()) {
        let got = run_case(&steps);
        prop_assert_eq!(got, Ok(()));
    }
}

fn run_case(steps: &[Vec<usize>]) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let world = runtime.block_on(start_world())?;
    let script = script_of(steps);
    let wanted = script.len();
    world.script(script);
    let outcome = runtime
        .block_on(world.send(Device::Phone, "go"))
        .map_err(|e| format!("{e:?}"))?;
    let task = runner_test::turn_helpers::task_of(&outcome);
    let events = world.task_events(task);
    let kinds: Vec<KnownTaskEventKind> = events
        .iter()
        .filter_map(|event| match &event.kind {
            TaskEventKind::Known(kind) => Some(kind.clone()),
            TaskEventKind::Raw(_) => None,
        })
        .collect();
    check_requests(&world, wanted)?;
    check_context_pairs(&kinds)?;
    runner_test::turn_loop1_checks::one_result_each(&kinds)?;
    runner_test::turn_loop1_checks::exec_order(&events)?;
    check_replay(&events)
}

async fn start_world() -> Result<TurnWorld, String> {
    world_with(vec![("a.txt".to_owned(), "x".to_owned())]).await
}

fn check_requests(world: &TurnWorld, wanted: usize) -> Result<(), String> {
    let got = world.chat_requests().len();
    if got != wanted {
        return Err(format!("requests {got} wanted {wanted}"));
    }
    Ok(())
}

fn check_context_pairs(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    for (index, kind) in kinds.iter().enumerate() {
        let KnownTaskEventKind::ModelAsked { .. } = kind else {
            continue;
        };
        let Some(prev) = index.checked_sub(1) else {
            return Err("asked at head".to_owned());
        };
        if !matches!(
            kinds.get(prev),
            Some(KnownTaskEventKind::ContextBuilt { .. })
        ) {
            return Err("asked without build".to_owned());
        }
    }
    Ok(())
}

fn check_replay(events: &[TaskEvent]) -> Result<(), String> {
    let full = anybot_domain::replay(events).map_err(|e| format!("{e:?}"))?;
    let Some(mid) = events.len().checked_div(2) else {
        return Err("no events".to_owned());
    };
    let Some(head) = events.get(..mid) else {
        return Err("slice head".to_owned());
    };
    let Some(tail) = events.get(mid..) else {
        return Err("slice tail".to_owned());
    };
    let snapshot = anybot_domain::snapshot_at(head, mid as u64);
    let view = anybot_domain::replay_from(&snapshot, tail).map_err(|e| format!("{e:?}"))?;
    if view != full {
        return Err(format!("replay mismatch {:?}", state_names(&view, &full)));
    }
    Ok(())
}

fn state_names(view: &TaskView, full: &TaskView) -> (String, String) {
    (format!("{:?}", view.status), format!("{:?}", full.status))
}
