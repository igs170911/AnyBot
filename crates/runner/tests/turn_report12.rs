use anybot_context as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use proptest::prelude::*;

use anybot_protocol::{DeliveryReportRecord, KnownTaskEventKind, TaskEvent};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{task_kinds, task_of};
use runner_test::turn_tool_helpers::all_specs;
use test_harness::{Step, ToolCallScript, model_replies_text, model_tool_calls};

fn report_setup() -> WorldSetup {
    WorldSetup {
        tools: all_specs(),
        scripts: vec![
            ("cat a.txt".to_owned(), CommandScript::ok("hi")),
            ("ls".to_owned(), CommandScript::ok("listed")),
        ],
        ..WorldSetup::default()
    }
}

fn arb_picks() -> BoxedStrategy<Vec<usize>> {
    prop::collection::vec(0usize..3usize, 1usize..3usize).boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn t_turn_loop_12(picks in arb_picks()) {
        let got = run_report_case(&picks);
        prop_assert_eq!(got, Ok(()));
    }
}

fn run_report_case(picks: &[usize]) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("runtime: {e}"))?;
    let world = runtime.block_on(TurnWorld::start(&report_setup()))?;
    world.script(picks_script(picks));
    let outcome = runtime
        .block_on(world.send(Device::Phone, "work"))
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let events = world.task_events(task);
    let first = report_twice(&events)?;
    let second = report_twice(&events)?;
    if first != second {
        return Err("two reports differ".to_owned());
    }
    check_report_shape(&world, task, &first)
}

fn picks_script(picks: &[usize]) -> Vec<Step> {
    let mut calls: Vec<ToolCallScript> = Vec::new();
    for (index, pick) in picks.iter().enumerate() {
        let slot = *pick;
        let name = ["edit_file", "read_file", "shell"]
            .get(slot)
            .copied()
            .unwrap_or("read_file");
        let args = [
            "{\"path\":\"a.txt\",\"old_text\":\"x\",\"new_text\":\"y\"}",
            "{\"path\":\"a.txt\"}",
            "{\"line\":\"ls\"}",
        ];
        calls.push(ToolCallScript {
            name: name.to_owned(),
            arguments: args.get(slot).copied().unwrap_or("{}").to_owned(),
            id: Some(format!("p{index}")),
            argument_parts: Vec::new(),
        });
    }
    vec![
        model_tool_calls(calls, "tool_calls", false),
        model_replies_text("done", true),
    ]
}

fn report_twice(events: &[TaskEvent]) -> Result<Vec<u8>, String> {
    let mut bytes: Option<Vec<u8>> = None;
    for _ in 0..2 {
        let report = anybot_trust::delivery_report(events, "the goal", Some("done"))
            .map_err(|e| format!("{e:?}"))?;
        let text = serde_json::to_vec(&report).map_err(|e| format!("{e}"))?;
        if let Some(old) = &bytes
            && *old != text
        {
            return Err("two reports differ".to_owned());
        }
        bytes = Some(text);
    }
    bytes.ok_or("no bytes".to_owned())
}

fn check_report_shape(
    world: &TurnWorld,
    task: anybot_foundation::TaskId,
    bytes: &[u8],
) -> Result<(), String> {
    let report: DeliveryReportRecord = serde_json::from_slice(bytes).map_err(|e| format!("{e}"))?;
    let view = anybot_domain::replay(&world.task_events(task)).map_err(|e| format!("{e:?}"))?;
    if report.status != view.status {
        return Err(format!("status {:?} vs {:?}", report.status, view.status));
    }
    if report.detail != view.detail {
        return Err(format!("detail {:?} vs {:?}", report.detail, view.detail));
    }
    let ids = evidence_ids_of(world, task);
    for step in &report.steps {
        let Some(first) = step.evidence_ids.first() else {
            continue;
        };
        if !ids.contains(first) {
            return Err(format!("step {} links foreign evidence", step.name));
        }
    }
    Ok(())
}

fn evidence_ids_of(
    world: &TurnWorld,
    task: anybot_foundation::TaskId,
) -> Vec<anybot_foundation::EvidenceId> {
    let mut out: Vec<anybot_foundation::EvidenceId> = Vec::new();
    for kind in task_kinds(world, &task) {
        let KnownTaskEventKind::ActionVerified { evidence_ids, .. } = kind else {
            continue;
        };
        out.extend(evidence_ids.iter());
    }
    out
}
