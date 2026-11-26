use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::sync::Arc;

use anybot_protocol::KnownTaskEventKind;
use proptest::prelude::*;
use runner_test::mem_store::MemoryRecord;
use runner_test::turn::{Device, TurnWorld, WorldSetup, tool_spec};
use runner_test::turn_helpers::{probe_of, task_of};
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

const NAMES: [&str; 3] = ["read_file", "shell", "edit_file"];

fn name_of(index: usize) -> &'static str {
    match index {
        1 => NAMES[1],
        0 | 2 | 3.. => NAMES[0],
    }
}

fn script_of(middles: &[Vec<usize>]) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut counter = 0usize;
    for middle in middles {
        let mut calls: Vec<ToolCallScript> = Vec::new();
        for name_index in middle {
            counter = counter.checked_add(1).unwrap_or(counter);
            calls.push(ToolCallScript {
                name: name_of(*name_index).to_owned(),
                arguments: "{\"path\":\"a.txt\"}".to_owned(),
                id: Some(format!("p{counter}")),
                argument_parts: Vec::new(),
            });
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
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn turn_ask_record_precedes_request(middles in arb_middles()) {
        let got = run_case(&middles);
        prop_assert_eq!(got, Ok(()));
    }
}

fn run_case(middles: &[Vec<usize>]) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let world = runtime.block_on(TurnWorld::start(&world_setup()))?;
    world.script(script_of(middles));
    let outcome = runtime
        .block_on(world.send(Device::Phone, "go"))
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    check_counts(&world, &task, middles.len().checked_add(1).unwrap_or(0))?;
    check_probe(&world.record, &task)?;
    check_order(&world, &task)
}

fn world_setup() -> WorldSetup {
    WorldSetup {
        probe_requests: true,
        tools: vec![
            tool_spec("read_file", "Read a file"),
            tool_spec("shell", "Run a shell command"),
            tool_spec("edit_file", "Edit a file"),
        ],
        ..WorldSetup::default()
    }
}

fn check_counts(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    steps: usize,
) -> Result<(), String> {
    let asked = count_kind(world, task, "model_asked");
    if world.chat_requests().len() != asked || asked != steps {
        return Err(format!(
            "counts: requests {} asked {} steps {}",
            world.chat_requests().len(),
            asked,
            steps
        ));
    }
    Ok(())
}

fn count_kind(world: &TurnWorld, task: &anybot_foundation::TaskId, name: &str) -> usize {
    world
        .kinds(*task)
        .iter()
        .filter(|k| k.kind_name() == name)
        .count()
}

fn check_probe(record: &Arc<MemoryRecord>, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let asked = count_asked(record, task);
    for index in 0..asked {
        let Some(value) = probe_of(record, task, "model_asked", index) else {
            return Err(format!("no probe at {index}"));
        };
        let want = u64::try_from(index).unwrap_or(u64::MAX);
        let got = u64::try_from(value).unwrap_or(u64::MAX);
        if got != want {
            return Err(format!("probe {index} is {value}"));
        }
    }
    Ok(())
}

fn count_asked(record: &Arc<MemoryRecord>, task: &anybot_foundation::TaskId) -> usize {
    let mut count = 0usize;
    for (row, _probe) in record.entries() {
        let anybot_runner::StoredBody::Task {
            task: owner, event, ..
        } = &row.body
        else {
            continue;
        };
        if *owner == *task && event.kind.kind_name() == "model_asked" {
            count = count.checked_add(1).unwrap_or(count);
        }
    }
    count
}

fn check_order(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<(), String> {
    let kinds = world.kinds(*task);
    let asked: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_i, k)| k.kind_name() == "model_asked")
        .map(|(i, _k)| i)
        .collect();
    let built: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_i, k)| k.kind_name() == "context_built")
        .map(|(i, _k)| i)
        .collect();
    let mut prev: usize = 0;
    for want in &asked {
        let between: Vec<usize> = built
            .iter()
            .copied()
            .filter(|b| *b > prev && *b < *want)
            .collect();
        if between.len() != 1 {
            return Err(format!("asks {want} has {} builds", between.len()));
        }
        prev = *want;
    }
    if built.first().copied().unwrap_or(usize::MAX) >= asked.first().copied().unwrap_or(usize::MAX)
    {
        return Err("no build before first ask".to_owned());
    }
    check_tool_pairs(&kinds)
}

fn check_tool_pairs(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    let mut requests: Vec<anybot_foundation::CallId> = Vec::new();
    let mut not_run: Vec<anybot_foundation::CallId> = Vec::new();
    for kind in kinds {
        if let KnownTaskEventKind::ModelAnswer { tool_requests, .. } = kind {
            for request in tool_requests {
                requests.push(request.call_id);
            }
        }
        if let KnownTaskEventKind::ToolNotRun { call_id, .. } = kind {
            not_run.push(*call_id);
        }
    }
    if requests != not_run {
        return Err(format!(
            "requests {} not_run {}",
            requests.len(),
            not_run.len()
        ));
    }
    Ok(())
}

use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use sha2 as _;
