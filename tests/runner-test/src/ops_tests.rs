use std::sync::Arc;

use anybot_foundation::{ContentDigest, TaskId, TurnId};
use anybot_protocol::{
    ActionOutcome, BeforeDigest, ComputerPlatform, EvidencePayload, VerificationResult,
};
use anybot_trust::{
    ActionContext, ActionRequest, ActionSubject, GateConfig, GatedComputer, PathView, RunStep,
};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::fake_scripts::CommandScript;
use crate::test_view::TestView;

fn fresh_clock() -> Arc<FakeClock> {
    FakeClock::new(1_000)
}

fn context(tool: &str, clock: &Arc<FakeClock>) -> ActionContext {
    ActionContext {
        task: TaskId::generate(clock.as_ref()),
        turn: TurnId::generate(clock.as_ref()),
        evidence_no: 7,
        computer: "fake-computer".to_owned(),
        workdir: "/workspace".to_owned(),
        tool: tool.to_owned(),
        classes: Vec::new(),
    }
}

fn request(tool: &str, subject: ActionSubject, clock: &Arc<FakeClock>) -> ActionRequest {
    ActionRequest {
        context: context(tool, clock),
        subject,
    }
}

fn run(computer: &FakeComputer, req: &ActionRequest) -> RunStep {
    let clock = computer.clock();
    run_action(
        req,
        computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    )
}

use anybot_trust::run_action;

fn bot_computer(clock: &Arc<FakeClock>) -> FakeComputer {
    FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(clock))
        .build()
}

fn write_request(content: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "edit_file",
        ActionSubject::WriteFile {
            path: PathView::raw("notes.md"),
            content: content.to_owned(),
        },
        clock,
    )
}

fn read_request(path: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "read_file",
        ActionSubject::ReadFile {
            path: PathView::raw(path),
        },
        clock,
    )
}

fn command_request(line: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "shell",
        ActionSubject::Command {
            line: line.to_owned(),
            timeout_ms: 30_000,
        },
        clock,
    )
}

fn first_payload(step: &RunStep) -> EvidencePayload {
    let run = step.clone().into_run();
    let Some(record) = run.evidence.first() else {
        panic!("no evidence in {run:?}");
    };
    record.payload.clone()
}

#[test]
fn test_fake_computer_write_then_read_returns_matching_content_digests() {
    let clock = fresh_clock();
    let computer = bot_computer(&clock);
    let written = run(&computer, &write_request("hello world", &clock));
    let EvidencePayload::Changed {
        path,
        before,
        after,
        size,
    } = first_payload(&written)
    else {
        panic!("changed payload missing");
    };
    assert_eq!(path, "/workspace/notes.md");
    assert_eq!(before, BeforeDigest::Absent);
    assert_eq!(after, ContentDigest::sha256(b"hello world"));
    assert_eq!(size, 11);
    let read = run(&computer, &read_request("notes.md", &clock)).into_run();
    let EvidencePayload::Seen { digest, .. } = read.evidence.first().unwrap().payload.clone()
    else {
        panic!("seen payload missing");
    };
    assert_eq!(digest, after);
    let verification = read.evidence.first().unwrap().verification.as_ref();
    assert!(verification.is_none(), "read verified {verification:?}");
}

#[test]
fn test_edit_file_evidence_carries_a_passing_reread_verification() {
    let clock = fresh_clock();
    let computer = bot_computer(&clock);
    let run = run(&computer, &write_request("hello world", &clock)).into_run();
    let record = run.evidence.first().unwrap();
    let Some(verification) = &record.verification else {
        panic!("verification missing");
    };
    assert_eq!(
        verification.result,
        VerificationResult::Passed,
        "detail {}",
        verification.detail
    );
    assert_eq!(record.outcome, ActionOutcome::Completed);
    assert_eq!(
        verification.result,
        VerificationResult::Passed,
        "detail {}",
        verification.detail
    );
    assert_eq!(
        verification.source,
        anybot_protocol::JudgementSource::ProgramCheck,
    );
    assert_eq!(
        computer.file_digest("notes.md"),
        Some(ContentDigest::sha256(b"hello world"))
    );
}

#[test]
fn test_fake_computer_command_without_a_script_reports_an_operation_failure() {
    let clock = fresh_clock();
    let computer = bot_computer(&clock);
    let run = run(&computer, &command_request("pwd", &clock)).into_run();
    assert_eq!(run.model.code, "tool-execution-failed");
    assert_eq!(run.evidence.first().unwrap().outcome, ActionOutcome::Failed);
    assert_eq!(run.summary.status.code, "turn-tool-failed");
    assert!(computer.logged().iter().any(|e| e == "run_command pwd"));
}

#[test]
fn test_fake_computer_unreachable_produces_no_evidence_and_no_operations() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .unreachable(true)
        .build();
    let run = run(&computer, &write_request("hello", &clock)).into_run();
    assert_eq!(run.evidence.len(), 0);
    assert_eq!(computer.logged(), vec!["write_file notes.md".to_owned()]);
    assert_eq!(run.summary.status.code, "turn-not-executed");
    assert_eq!(
        run.summary.status.params.first().map(|(_, v)| v.as_str()),
        Some("computer-unreachable"),
    );
}

#[test]
fn test_fake_computer_command_duration_follows_the_fake_clock() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("ls -la", CommandScript::ok("files").with_duration(250))
        .build();
    let run = run(&computer, &command_request("ls -la", &clock)).into_run();
    let EvidencePayload::Executed {
        exit_code,
        elapsed_ms,
        ..
    } = run.evidence.first().unwrap().payload.clone()
    else {
        panic!("executed payload missing");
    };
    assert_eq!(elapsed_ms, 250);
    assert_eq!(exit_code, Some(0));
    assert_eq!(clock.now_ms(), 1_250);
    assert_eq!(
        run.evidence.first().unwrap().outcome,
        ActionOutcome::Completed
    );
}

#[test]
fn test_command_with_a_nonzero_exit_code_is_recorded_as_failed() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("check.sh", CommandScript::failing("nope", 3))
        .build();
    let run = run(&computer, &command_request("check.sh", &clock)).into_run();
    let record = run.evidence.first().unwrap();
    let EvidencePayload::Executed { exit_code, .. } = record.payload.clone() else {
        panic!("executed payload missing");
    };
    assert_eq!(exit_code, Some(3));
    assert_eq!(record.outcome, ActionOutcome::Failed);
    assert_eq!(run.summary.status.code, "turn-tool-failed");
    assert_eq!(record.verification.as_ref().unwrap().detail, "exit code 3");
}

#[test]
fn test_floor_denied_action_never_reaches_the_fake_computer() {
    let clock = fresh_clock();
    let computer = bot_computer(&clock);
    let run = run(&computer, &command_request("sudo rm -rf /", &clock)).into_run();
    assert_eq!(computer.logged(), Vec::<String>::new());
    assert_eq!(run.evidence.len(), 1);
    let record = run.evidence.first().unwrap();
    assert_eq!(record.outcome, ActionOutcome::Rejected);
    assert_eq!(run.model.code, "gate-denied");
    assert_eq!(run.gate.source, anybot_protocol::JudgementSource::Rule);
}

#[test]
fn test_fake_computer_records_operations_in_the_order_it_receives_them() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .file("a.txt", "one")
        .script("cat a.txt", CommandScript::ok("one"))
        .build();
    let first = run(&computer, &read_request("a.txt", &clock)).into_run();
    let second = run(&computer, &command_request("cat a.txt", &clock)).into_run();
    assert_eq!(first.evidence.len(), 1);
    assert_eq!(second.evidence.len(), 1);
    assert_eq!(
        computer.logged(),
        vec![
            "read_file a.txt".to_owned(),
            "run_command cat a.txt".to_owned(),
            "run_command done wakes 0".to_owned(),
        ],
    );
}

#[test]
fn test_fake_computer_declares_platform_caps_at_build_time() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .platform(ComputerPlatform::Windows)
        .without_sandbox()
        .clock(Arc::clone(&clock))
        .build();
    let caps = computer.declared_caps();
    assert_eq!(caps.platform, ComputerPlatform::Windows);
    assert_eq!(caps.command_sandbox, anybot_protocol::CapState::Unsupported);
}
