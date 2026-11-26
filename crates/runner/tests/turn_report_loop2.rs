use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{
    EvidencePayload, KnownTaskEventKind, ReportOutcome, ReportSummary, TaskDetail, TaskStatus,
};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{delivery_of, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call, evidence_record};
use test_harness::{Step, model_replies_text, model_tool_calls};

#[tokio::test]
async fn t_turn_loop_2() -> Result<(), String> {
    let setup = report_setup();
    let world = TurnWorld::start(&setup).await?;
    world.script(loop2_script());
    let outcome = world.send(Device::Desktop, "make a.txt then read it").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    if world.chat_requests().len() != 3 {
        return Err(format!("requests {}", world.chat_requests().len()));
    }
    if world.tree_content("a.txt").as_deref() != Some("hi") {
        return Err("a.txt content is not hi".to_owned());
    }
    let edit = evidence_record(&world, &task, "a.txt")?;
    check_edit_payload(&edit)?;
    check_shell_payload(&world, task)?;
    check_loop2_status(&world, task)?;
    check_report(&world, task)
}

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

fn loop2_script() -> Vec<Step> {
    vec![
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"hi\"}",
                "c1",
            )],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("shell", "{\"line\":\"cat a.txt\"}", "c2")],
            "tool_calls",
            false,
        ),
        model_replies_text("done", true),
    ]
}

fn check_edit_payload(record: &anybot_protocol::EvidenceRecord) -> Result<(), String> {
    let EvidencePayload::Changed { after, .. } = &record.payload else {
        return Err("edit evidence is not changed".to_owned());
    };
    let wanted = runner_test::turn_helpers::sha256_hex(b"hi");
    if after.hex() != wanted {
        return Err(format!("after {} wanted {}", after.hex(), wanted));
    }
    let Some(verification) = &record.verification else {
        return Err("edit evidence has no verification".to_owned());
    };
    if verification.source != anybot_protocol::JudgementSource::ProgramCheck {
        return Err(format!("edit source {:?}", verification.source));
    }
    Ok(())
}

fn check_shell_payload(world: &TurnWorld, task: anybot_foundation::TaskId) -> Result<(), String> {
    for kind in task_kinds(world, &task) {
        let KnownTaskEventKind::EvidenceAdded { record } = kind else {
            continue;
        };
        if matches!(
            record.payload,
            EvidencePayload::Executed {
                exit_code: Some(0),
                ..
            }
        ) {
            return Ok(());
        }
    }
    Err("no exit 0 evidence".to_owned())
}

fn check_loop2_status(world: &TurnWorld, task: anybot_foundation::TaskId) -> Result<(), String> {
    let state = world.task_state(task)?;
    if state.0 != TaskStatus::Completed {
        return Err(format!("status {:?}", state.0));
    }
    if state.1 != Some(TaskDetail::Accepted) {
        return Err(format!("detail {:?}", state.1));
    }
    Ok(())
}

fn check_report(world: &TurnWorld, task: anybot_foundation::TaskId) -> Result<(), String> {
    let report = delivery_of(world, task)?;
    if report.summary != ReportSummary::Text("done".to_owned()) {
        return Err(format!("summary {:?}", report.summary));
    }
    if report.steps.len() != 2 {
        return Err(format!("steps {}", report.steps.len()));
    }
    if report
        .steps
        .iter()
        .any(|s| s.outcome != ReportOutcome::Completed)
    {
        return Err("a step is not completed".to_owned());
    }
    Ok(())
}
