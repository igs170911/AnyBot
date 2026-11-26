use std::sync::Arc;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::fake_scripts::CommandScript;
use crate::ops_tests::{bot_computer, command_request, fresh_clock, write_request};
use crate::test_view::TestView;
use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, JudgementSource, VerificationResult,
};
use anybot_trust::{ActionRun, GateStep, GatedComputer, RequestedAction, exec_step, gate_step};

fn gate_and_exec(computer: &FakeComputer, req: &anybot_trust::ActionRequest) -> ActionRun {
    let clock = computer.clock();
    let step = gate_step(
        req,
        computer,
        &anybot_trust::GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    );
    match step {
        GateStep::Allow(ticket) => exec_step(req, ticket, computer, &TestView, clock.as_ref()),
        GateStep::Denied(run) => run,
        GateStep::Ask(ticket) => {
            panic!("ask step returned for a bot-machine write: {ticket:?}");
        }
    }
}

#[test]
fn t_trust_pipeline_1() -> Result<(), String> {
    let clock = fresh_clock();
    let computer = bot_computer(&clock);
    let req = write_request("A", &clock);
    let run = gate_and_exec(&computer, &req);
    let Some(record) = run.evidence.first() else {
        return Err("no evidence".to_owned());
    };
    if record.kind != EvidenceKind::Changed {
        return Err(format!("kind {:?}", record.kind));
    }
    if record.gate.source != JudgementSource::Rule {
        return Err(format!("source {:?}", record.gate.source));
    }
    if record.computer != computer.name() {
        return Err(format!("computer {}", record.computer));
    }
    let EvidencePayload::Changed { before, after, .. } = &record.payload else {
        return Err("not changed".to_owned());
    };
    match before {
        anybot_protocol::BeforeDigest::Absent => {}
        anybot_protocol::BeforeDigest::Present(old) => {
            if old == after {
                return Err("unchanged digest".to_owned());
            }
        }
    }
    let Some(verification) = &record.verification else {
        return Err("no verification".to_owned());
    };
    if verification.result != VerificationResult::Passed {
        return Err(format!("verify {:?}", verification.result));
    }
    if verification.source != JudgementSource::ProgramCheck {
        return Err(format!("verify source {:?}", verification.source));
    }
    Ok(())
}

#[test]
fn t_trust_verify_1() -> Result<(), String> {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .script("exit 3", CommandScript::failing("", 3))
        .clock(Arc::clone(&clock))
        .build();
    let edit = anybot_trust::ActionRequest {
        context: crate::ops_tests::context("edit_file", &clock),
        subject: RequestedAction::WriteFile {
            path: "r.txt".to_owned(),
            content: "content".to_owned(),
        },
    };
    let edit_run = gate_and_exec(&computer, &edit);
    check_edit_verified(&edit_run)?;
    let command = command_request("exit 3", &clock);
    let command_run = gate_and_exec(&computer, &command);
    check_command_exit(&command_run)?;
    Ok(())
}

fn check_edit_verified(run: &ActionRun) -> Result<(), String> {
    let Some(record) = run.evidence.first() else {
        return Err("no edit evidence".to_owned());
    };
    let Some(verification) = &record.verification else {
        return Err("no edit verification".to_owned());
    };
    if verification.result != VerificationResult::Passed {
        return Err(format!("edit verify {:?}", verification.result));
    }
    if verification.source != JudgementSource::ProgramCheck {
        return Err(format!("edit source {:?}", verification.source));
    }
    Ok(())
}

fn check_command_exit(run: &ActionRun) -> Result<(), String> {
    let Some(record) = run.evidence.first() else {
        return Err("no command evidence".to_owned());
    };
    let EvidencePayload::Executed { exit_code, .. } = &record.payload else {
        return Err("not executed".to_owned());
    };
    if *exit_code != Some(3) {
        return Err(format!("exit {exit_code:?}"));
    }
    if run.summary.status.code != anybot_trust::CODE_TURN_TOOL_FAILED {
        return Err(format!("status {}", run.summary.status.code));
    }
    if record.outcome != ActionOutcome::Failed {
        return Err(format!("outcome {:?}", record.outcome));
    }
    Ok(())
}
