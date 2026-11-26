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
use test_harness as _;
use thiserror as _;
use tokio as _;

use anybot_protocol::{EvidenceKind, EvidencePayload, KnownTaskEventKind, TaskDetail, TaskStatus};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    delivery_of, fmt_text, kind_names, messages_of, request_body, state_of, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{all_specs, edit_call, one_tool_step};
use test_harness::{model_replies_text, model_tool_calls};

fn result_call_id(kind: &KnownTaskEventKind) -> Option<String> {
    match kind {
        KnownTaskEventKind::ToolIntent { call_id, .. }
        | KnownTaskEventKind::GateResult { call_id, .. }
        | KnownTaskEventKind::ToolNotRun { call_id, .. } => Some(call_id.as_str().clone()),
        KnownTaskEventKind::TaskCreated { .. }
        | KnownTaskEventKind::TaskCancelled { .. }
        | KnownTaskEventKind::TaskStopped
        | KnownTaskEventKind::Wake { .. }
        | KnownTaskEventKind::GoalPaused { .. }
        | KnownTaskEventKind::ComputerOffline
        | KnownTaskEventKind::ModelUnreachable
        | KnownTaskEventKind::Stalled { .. }
        | KnownTaskEventKind::TurnStarted { .. }
        | KnownTaskEventKind::TurnEnded { .. }
        | KnownTaskEventKind::ContextBuilt { .. }
        | KnownTaskEventKind::ModelAsked { .. }
        | KnownTaskEventKind::ModelAnswer { .. }
        | KnownTaskEventKind::ModelUsage { .. }
        | KnownTaskEventKind::ModelSwapped { .. }
        | KnownTaskEventKind::ToolStarted { .. }
        | KnownTaskEventKind::ToolCompleted { .. }
        | KnownTaskEventKind::ToolFailed { .. }
        | KnownTaskEventKind::OutcomeUnknown { .. }
        | KnownTaskEventKind::Reconciled { .. }
        | KnownTaskEventKind::ActionVerified { .. }
        | KnownTaskEventKind::EvidenceAdded { .. }
        | KnownTaskEventKind::CriterionPassed { .. }
        | KnownTaskEventKind::CriterionFailed { .. }
        | KnownTaskEventKind::AcceptanceUnavailable
        | KnownTaskEventKind::ClaimChecked { .. }
        | KnownTaskEventKind::BudgetUsage { .. }
        | KnownTaskEventKind::BudgetLimit { .. }
        | KnownTaskEventKind::CardShown { .. }
        | KnownTaskEventKind::CardAnswered { .. }
        | KnownTaskEventKind::DeviceReport { .. }
        | KnownTaskEventKind::DeliveryReport { .. }
        | KnownTaskEventKind::DoneWhenChange { .. }
        | KnownTaskEventKind::DoneWhenAppliedByUser { .. } => None,
    }
}

fn shell_call(line: &str, id: &str) -> test_harness::ToolCallScript {
    runner_test::turn_tool_helpers::call("shell", &format!("{{\"line\":\"{line}\"}}"), id)
}

#[tokio::test]
async fn turn_middle_denied_blocks_next_call() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("ls".to_owned(), CommandScript::ok("listed"))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![
                shell_call("ls", "c1"),
                shell_call("sudo rm -rf /", "c2"),
                shell_call("ls", "c3"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let names = kind_names(&world, &task);
    let not_runs = names.iter().filter(|name| *name == "tool_not_run").count();
    if not_runs != 1 {
        return Err(format!("not run count {not_runs}"));
    }
    let body = request_body(&world.model, 1)?;
    let messages = messages_of(&body);
    if messages.len() < 3 {
        return Err(format!("messages {} below 3", messages.len()));
    }
    let mut ids: Vec<String> = Vec::new();
    for kind in task_kinds(&world, &task) {
        let Some(call_id) = result_call_id(&kind) else {
            continue;
        };
        if !ids.contains(&call_id) {
            ids.push(call_id);
        }
    }
    let third = runner_test::turn_tool_helpers::tool_message_content(
        &messages,
        ids.get(2)
            .map(String::as_str)
            .ok_or("no third id".to_owned())?,
    )?;
    let wanted = fmt_text(
        anybot_messages::Lang::En,
        "turn-not-executed",
        &anybot_messages::Params::new().with_text("reason", "previous-refused"),
    )?;
    if !third.contains(&wanted) {
        return Err(format!("third text {third}"));
    }
    Ok(())
}

#[tokio::test]
async fn turn_ask_action_is_not_executed() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![shell_call(
        "git push origin main",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let names = kind_names(&world, &task);
    if names.contains(&"tool_started".to_owned()) {
        return Err("git push was executed".to_owned());
    }
    let mut found = false;
    for kind in task_kinds(&world, &task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if record.kind != EvidenceKind::Rejected {
            continue;
        }
        let EvidencePayload::Rejected { reason } = &record.payload else {
            continue;
        };
        if reason.code == "gate-ask-unavailable" {
            found = true;
        }
    }
    if !found {
        return Err("no ask-unavailable rejection evidence".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn turn_last_write_failure_leaves_unaccepted() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![("make".to_owned(), CommandScript::failing("build log", 2))],
        files: vec![("a.txt".to_owned(), "x".to_owned())],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"a.txt\",\"old_text\":\"x\",\"new_text\":\"y\"}",
                "c1",
            )],
            "tool_calls",
            false,
        ),
        model_tool_calls(vec![shell_call("make", "c2")], "tool_calls", false),
        model_replies_text("done", false),
    ]);
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let state = state_of(&world, &task)?;
    if state.0 != TaskStatus::Unaccepted {
        return Err(format!("status {:?}", state.0));
    }
    if state.1 != Some(TaskDetail::AcceptanceFailed) {
        return Err(format!("detail {:?}", state.1));
    }
    let report = delivery_of(&world, task)?;
    if report.status != TaskStatus::Unaccepted {
        return Err(format!("report status {:?}", report.status));
    }
    if !report
        .attention
        .iter()
        .any(|item| item.args_summary.contains("make"))
    {
        return Err(format!("attention {:?}", report.attention));
    }
    Ok(())
}

#[tokio::test]
async fn turn_reread_digest_mismatch_leaves_unaccepted() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.gated.set_mutate_after_write(true);
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"hi\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let state = state_of(&world, &task)?;
    if state.0 == TaskStatus::Completed {
        return Err("status completed despite digest mismatch".to_owned());
    }
    if state.0 != TaskStatus::Unaccepted || state.1 != Some(TaskDetail::AcceptanceFailed) {
        return Err(format!("state {state:?}"));
    }
    let report = delivery_of(&world, task)?;
    if report.status != TaskStatus::Unaccepted {
        return Err(format!("report status {:?}", report.status));
    }
    Ok(())
}
