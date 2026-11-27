use std::sync::Arc;

use anybot_foundation::CallId;
use anybot_protocol::{EvidencePayload, GateOutcome, JudgementSource};
use anybot_trust::GatedComputer;
use anybot_trust::{GateAnswer, GateConfig, RequestedAction, RunStep, answer_ask, run_action};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::fake_scripts::CommandScript;
use crate::ops_support::{command_request, fresh_clock, run};
use crate::test_view::TestView;

#[test]
fn test_a_forged_resolved_path_cannot_hide_the_path_the_computer_uses() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .build();
    let request = anybot_trust::ActionRequest {
        context: crate::ops_support::context("edit_file", &clock),
        subject: RequestedAction::WriteFile {
            path: "/Users/me/.zshrc".to_owned(),
            content: "alias".to_owned(),
        },
    };
    let step = run_action(
        &request,
        &anybot_trust::CallContext::new(
            CallId::generate(clock.as_ref()),
            anybot_foundation::TurnId::generate(clock.as_ref()),
            "c1",
        ),
        &computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    );
    assert!(matches!(step, RunStep::Denied(_)));
    assert!(computer.file_content("/Users/me/.zshrc").is_none());
    assert_eq!(computer.logged().len(), 0);
}

#[test]
fn test_a_workspace_symlink_to_the_outside_is_judged_as_outside() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .link("/workspace/notes.md", "/Users/me/.zshrc")
        .build();
    let request = anybot_trust::ActionRequest {
        context: crate::ops_support::context("edit_file", &clock),
        subject: RequestedAction::WriteFile {
            path: "notes.md".to_owned(),
            content: "alias".to_owned(),
        },
    };
    let step = run_action(
        &request,
        &anybot_trust::CallContext::new(
            CallId::generate(clock.as_ref()),
            anybot_foundation::TurnId::generate(clock.as_ref()),
            "c1",
        ),
        &computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    );
    assert!(matches!(step, RunStep::Denied(_)));
    assert!(computer.file_content("/Users/me/.zshrc").is_none());
}

#[test]
fn test_a_symlink_swapped_between_gate_and_execution_is_refused() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .user_machine()
        .files()
        .clock(Arc::clone(&clock))
        .link("/workspace/notes.md", "/workspace/notes.md")
        .build();
    let request = anybot_trust::ActionRequest {
        context: crate::ops_support::context("edit_file", &clock),
        subject: RequestedAction::WriteFile {
            path: "notes.md".to_owned(),
            content: "kept".to_owned(),
        },
    };
    let step = run_action(
        &request,
        &anybot_trust::CallContext::new(
            CallId::generate(clock.as_ref()),
            anybot_foundation::TurnId::generate(clock.as_ref()),
            "c1",
        ),
        &computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    );
    let RunStep::Ask(ticket) = step else {
        panic!("the write must ask before the link moves");
    };
    computer.retarget_link("/workspace/notes.md", "/Users/me/.zshrc");
    let run = answer_ask(
        &ticket,
        GateAnswer::Allow,
        &computer,
        &TestView,
        clock.as_ref(),
    );
    assert_eq!(run.gate.outcome, GateOutcome::Deny);
    assert!(computer.file_content("/Users/me/.zshrc").is_none());
    assert!(computer.logged().is_empty());
}

fn push_computer(clock: &Arc<FakeClock>) -> FakeComputer {
    FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(clock))
        .script("git push origin", CommandScript::ok("done"))
        .build()
}

#[test]
fn test_an_ask_step_answered_allow_executes_the_asking_action() {
    let clock = fresh_clock();
    let computer = push_computer(&clock);
    let step = run(&computer, &command_request("git push origin", &clock));
    let RunStep::Ask(ticket) = step else {
        panic!("the sending action must ask");
    };
    assert_eq!(ticket.gate().outcome, GateOutcome::Ask);
    let run = answer_ask(
        &ticket,
        GateAnswer::Allow,
        &computer,
        &TestView,
        clock.as_ref(),
    );
    assert_eq!(run.gate.outcome, GateOutcome::Allow);
    assert_eq!(run.gate.source, JudgementSource::You);
    assert_eq!(computer.logged().len(), 2);
}

#[test]
fn test_an_ask_step_answered_deny_never_reaches_the_computer() {
    let clock = fresh_clock();
    let computer = push_computer(&clock);
    let step = run(&computer, &command_request("git push origin", &clock));
    let RunStep::Ask(ticket) = step else {
        panic!("the sending action must ask");
    };
    let run = answer_ask(
        &ticket,
        GateAnswer::Deny,
        &computer,
        &TestView,
        clock.as_ref(),
    );
    assert_eq!(run.gate.outcome, GateOutcome::Deny);
    assert_eq!(computer.logged().len(), 0);
}

#[test]
fn test_answering_an_ask_on_a_different_computer_refuses_the_action() {
    let clock = fresh_clock();
    let asking = push_computer(&clock);
    let step = run(&asking, &command_request("git push origin", &clock));
    let RunStep::Ask(ticket) = step else {
        panic!("the sending action must ask");
    };
    assert_eq!(ticket.computer_name(), asking.name());
    let answering = FakeComputerBuilder::default()
        .bot_machine()
        .named("other-machine")
        .clock(Arc::clone(&clock))
        .script("git push origin", CommandScript::ok("done"))
        .build();
    let run = answer_ask(
        &ticket,
        GateAnswer::Allow,
        &answering,
        &TestView,
        clock.as_ref(),
    );
    assert_eq!(run.gate.outcome, GateOutcome::Deny);
    assert_eq!(run.gate.source, JudgementSource::ProgramCheck);
    assert_eq!(answering.logged().len(), 0);
    assert_eq!(asking.logged().len(), 0);
    let EvidencePayload::Rejected { reason, .. } = &run.evidence.first().unwrap().payload else {
        panic!("the refused answer must carry a rejected payload");
    };
    assert_eq!(reason.code, "gate-ask-computer-mismatch");
}

#[test]
fn test_a_denied_action_produces_no_answer_ticket() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .build();
    let request = anybot_trust::ActionRequest {
        context: crate::ops_support::context("edit_file", &clock),
        subject: RequestedAction::WriteFile {
            path: "/tmp/outside.md".to_owned(),
            content: "x".to_owned(),
        },
    };
    let step = run_action(
        &request,
        &anybot_trust::CallContext::new(
            CallId::generate(clock.as_ref()),
            anybot_foundation::TurnId::generate(clock.as_ref()),
            "c1",
        ),
        &computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    );
    assert!(
        matches!(step, RunStep::Denied(_)),
        "the outside write must be denied, not asked"
    );
    assert_eq!(computer.logged().len(), 0);
}

#[test]
fn test_an_approved_command_runs_the_exact_line_that_was_written() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("Echo MixedCase", CommandScript::ok("out"))
        .build();
    let run = run(&computer, &command_request("Echo MixedCase", &clock)).into_run();
    let first = run.evidence.first().unwrap();
    let EvidencePayload::Executed { command, .. } = &first.payload else {
        panic!("the executed action must carry an executed payload");
    };
    assert_eq!(command, "Echo MixedCase");
    assert_eq!(
        computer.logged(),
        vec!["run_command echo mixedcase", "run_command done wakes 0"],
    );
}

#[cfg(test)]
#[path = "pipeline_flow_tests_b.rs"]
mod flow_b;
