use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;
use tokio as _;

use anybot_protocol::KnownTaskEventKind;
use proptest::prelude::*;
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::task_of;
use runner_test::turn_tool_helpers::all_specs;
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

const NAMES: [&str; 3] = ["read_file", "shell", "edit_file"];

fn name_of(index: usize) -> &'static str {
    match index {
        1 => NAMES[1],
        0 | 2 | 3.. => NAMES[0],
    }
}

fn args_of(name: &str, id: &str) -> ToolCallScript {
    let arguments = match name {
        "shell" => "{\"line\":\"ls\"}".to_owned(),
        "edit_file" => "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"z\"}".to_owned(),
        _ => "{\"path\":\"a.txt\"}".to_owned(),
    };
    ToolCallScript {
        name: name.to_owned(),
        arguments,
        id: Some(id.to_owned()),
        argument_parts: Vec::new(),
    }
}

fn script_of(middles: &[Vec<usize>]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut counter = 0usize;
    for middle in middles {
        let mut calls: Vec<ToolCallScript> = Vec::new();
        for name_index in middle {
            counter = counter.checked_add(1).unwrap_or(counter);
            calls.push(args_of(name_of(*name_index), &format!("q{counter}")));
        }
        steps.push(model_tool_calls(calls, "tool_calls", false));
    }
    steps.push(model_replies_text("done", false));
    steps
}

fn arb_middles() -> BoxedStrategy<Vec<Vec<usize>>> {
    prop::collection::vec(prop::collection::vec(0usize..3usize, 1usize..3usize), 0..3).boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn turn_intent_record_precedes_computer_operation(middles in arb_middles()) {
        let got = run_case(&middles);
        prop_assert_eq!(got, Ok(()));
    }
}

fn run_case(middles: &[Vec<usize>]) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let world = runtime.block_on(TurnWorld::start(&intent_setup()))?;
    world.script(script_of(middles));
    let outcome = runtime
        .block_on(world.send(Device::Phone, "go"))
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    check_intent_order(&world, &task)
}

fn intent_setup() -> WorldSetup {
    WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        scripts: vec![("ls".to_owned(), CommandScript::ok("listed"))],
        ..WorldSetup::default()
    }
}

fn check_intent_order(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let kinds: Vec<KnownTaskEventKind> = world.kinds(*task);
    let log = operations_of(&world.gated.logged());
    let started = started_ids(&kinds);
    if log.len() != started.len() {
        return Err(format!("log {} started {}", log.len(), started.len()));
    }
    let intents = intent_ids(&kinds);
    if intents != started {
        return Err(format!("intents {intents:?} started {started:?}"));
    }
    for call_id in &started {
        let intent_at = kinds
            .iter()
            .position(|k| matches!(k, KnownTaskEventKind::ToolIntent { call_id: c, .. } if c.as_str() == *call_id))
            .ok_or("intent missing".to_owned())?;
        let start_at = kinds
            .iter()
            .position(|k| matches!(k, KnownTaskEventKind::ToolStarted { call_id: c, .. } if c.as_str() == *call_id))
            .ok_or("start missing".to_owned())?;
        if intent_at >= start_at {
            return Err(format!(
                "intent {call_id} at {intent_at} start at {start_at}"
            ));
        }
    }
    Ok(())
}

fn operations_of(log: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in log {
        if entry == "capabilities" || entry.starts_with("run_command done") {
            continue;
        }
        out.push(entry.clone());
    }
    out
}

fn intent_ids(kinds: &[KnownTaskEventKind]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for kind in kinds {
        let KnownTaskEventKind::ToolIntent { call_id, .. } = kind else {
            continue;
        };
        ids.push(call_id.as_str().clone());
    }
    ids
}

fn started_ids(kinds: &[KnownTaskEventKind]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for kind in kinds {
        let KnownTaskEventKind::ToolStarted { call_id, .. } = kind else {
            continue;
        };
        ids.push(call_id.as_str().clone());
    }
    ids
}
