use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{EvidenceKind, EvidencePayload, KnownTaskEventKind};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, one_tool_step};

fn sandbox_setup() -> WorldSetup {
    WorldSetup {
        tools: all_specs(),
        scripts: vec![
            (
                "sudo apt-get install jq".to_owned(),
                CommandScript::ok("jq installed"),
            ),
            (
                "make test && pip install y".to_owned(),
                CommandScript::ok("built"),
            ),
        ],
        ..WorldSetup::default()
    }
}

fn shell_call(line: &str, id: &str) -> test_harness::ToolCallScript {
    runner_test::turn_tool_helpers::call("shell", &format!("{{\"line\":\"{line}\"}}"), id)
}

fn deny_reason(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Result<String, String> {
    for kind in task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if record.kind == EvidenceKind::Rejected {
            return Ok(record.payload.rejected_code());
        }
    }
    Err("no rejected evidence".to_owned())
}

trait EvidencePayloadExt {
    fn rejected_code(&self) -> String;
}

impl EvidencePayloadExt for EvidencePayload {
    fn rejected_code(&self) -> String {
        match self {
            EvidencePayload::Rejected { reason } => reason.code.clone(),
            EvidencePayload::Approved { .. }
            | EvidencePayload::Changed { .. }
            | EvidencePayload::Seen { .. }
            | EvidencePayload::Executed { .. }
            | EvidencePayload::NotExecuted { .. } => String::new(),
        }
    }
}

fn gate_call_ids(world: &TurnWorld, task: &anybot_foundation::TaskId) -> Vec<String> {
    let mut ids = Vec::new();
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::GateResult { call_id, .. } = kind {
            ids.push(call_id.as_str());
        }
    }
    ids
}

fn tool_text(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    gate_index: usize,
    request_index: usize,
) -> Result<String, String> {
    let body = request_body(&world.model, request_index)?;
    let ids = gate_call_ids(world, task);
    let id = ids
        .get(gate_index)
        .map(String::as_str)
        .ok_or("no gate id".to_owned())?;
    runner_test::turn_tool_helpers::tool_message_content(&messages_of(&body), id)
}

#[tokio::test]
async fn t_sandbox_deny_3() -> Result<(), String> {
    let world = TurnWorld::start(&sandbox_setup()).await?;
    world.script(one_tool_step(vec![
        shell_call("sudo apt-get install jq", "c1"),
        shell_call("sudo rm -rf /", "c2"),
    ]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    if deny_reason(&world, &task)? != "gate-command-denied" {
        return Err("deny code wrong".to_owned());
    }
    if !ran_evidence(&world, &task, "sudo apt-get install jq") {
        return Err("first command not run".to_owned());
    }
    let wanted = fmt_text(
        Lang::En,
        "gate-command-denied",
        &Params::new().with_text("command", "sudo rm -rf /"),
    )?;
    let second = tool_text(&world, &task, 1, 1)?;
    if !second.contains(&wanted) {
        return Err(format!("second text {second}"));
    }
    Ok(())
}

fn ran_evidence(world: &TurnWorld, task: &anybot_foundation::TaskId, command: &str) -> bool {
    for kind in task_kinds(world, task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if let EvidencePayload::Executed { command: got, .. } = &record.payload
            && got.as_str() == command
        {
            return true;
        }
    }
    false
}

#[tokio::test]
async fn t_sandbox_levels_4() -> Result<(), String> {
    let world = TurnWorld::start(&sandbox_setup()).await?;
    world.script(one_tool_step(vec![
        shell_call("make test && pip install y", "c1"),
        runner_test::turn_tool_helpers::edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"m\"}",
            "c2",
        ),
    ]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    if !ran_evidence(&world, &task, "make test && pip install y") {
        return Err("command not run".to_owned());
    }
    if runner_test::turn_tool_helpers::evidence_record(&world, &task, "a.txt").is_err() {
        return Err("edit evidence missing".to_owned());
    }
    let kinds = runner_test::turn_helpers::kind_names(&world, &task);
    if kinds.contains(&"tool_not_run".to_owned()) {
        return Err("not run recorded".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn t_files_n2_2() -> Result<(), String> {
    let world = TurnWorld::start(&sandbox_setup()).await?;
    world.script(one_tool_step(vec![shell_call("sudo rm -rf /", "c1")]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    if deny_reason(&world, &task)? != "gate-command-denied" {
        return Err("deny code wrong".to_owned());
    }
    let wanted = fmt_text(
        Lang::En,
        "gate-command-denied",
        &Params::new().with_text("command", "sudo rm -rf /"),
    )?;
    let first = tool_text(&world, &task, 0, 1)?;
    if !first.contains(&wanted) {
        return Err(format!("first text {first}"));
    }
    if world
        .gated
        .logged()
        .iter()
        .any(|line| line.contains("rm -rf /"))
    {
        return Err("command ran".to_owned());
    }
    Ok(())
}
