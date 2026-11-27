use std::sync::Arc;

use anybot_foundation::ContentDigest;
use anybot_protocol::{
    ActionOutcome, BeforeDigest, ComputerPlatform, EvidencePayload, VerificationResult,
};
use anybot_trust::GatedComputer;

use crate::fake_builder::FakeComputerBuilder;
use crate::ops_support::{
    bot_computer, command_request, fresh_clock, read_request, run, write_request,
};

fn first_payload(run: &anybot_trust::ActionRun) -> EvidencePayload {
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
    } = first_payload(&written.into_run())
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
    assert_eq!(
        computer.logged(),
        vec!["write_file /workspace/notes.md".to_owned()]
    );
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
        .script(
            "ls -la",
            crate::fake_scripts::CommandScript::ok("files").with_duration(250),
        )
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
        .script(
            "check.sh",
            crate::fake_scripts::CommandScript::failing("nope", 3),
        )
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
        .script("cat a.txt", crate::fake_scripts::CommandScript::ok("one"))
        .build();
    let first = run(&computer, &read_request("a.txt", &clock)).into_run();
    let second = run(&computer, &command_request("cat a.txt", &clock)).into_run();
    assert_eq!(first.evidence.len(), 1);
    assert_eq!(second.evidence.len(), 1);
    assert_eq!(
        computer.logged(),
        vec![
            "read_file /workspace/a.txt".to_owned(),
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
