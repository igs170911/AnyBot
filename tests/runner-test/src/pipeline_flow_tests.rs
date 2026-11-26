use std::sync::Arc;

use anybot_protocol::{EvidencePayload, GateOutcome, JudgementSource};
use anybot_trust::{
    ActionSubject, GateAnswer, GateConfig, PathView, RunStep, answer_ask, run_action,
};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::fake_scripts::CommandScript;
use crate::ops_tests::{command_request, fresh_clock, run};
use crate::test_view::TestView;

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
        *ticket,
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
        *ticket,
        GateAnswer::Deny,
        &computer,
        &TestView,
        clock.as_ref(),
    );
    assert_eq!(run.gate.outcome, GateOutcome::Deny);
    assert_eq!(computer.logged().len(), 0);
}

#[test]
fn test_a_denied_action_produces_no_answer_ticket() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .build();
    let request = anybot_trust::ActionRequest {
        context: crate::ops_tests::context("edit_file", &clock),
        subject: ActionSubject::WriteFile {
            path: PathView::raw("/tmp/outside.md"),
            content: "x".to_owned(),
        },
    };
    let step = run_action(
        &request,
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

#[test]
fn test_a_finished_run_leaves_nothing_to_ask_the_computer_again() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("ls", CommandScript::ok("files"))
        .script("pwd", CommandScript::ok("/workspace"))
        .build();
    let ls_run = run(&computer, &command_request("ls", &clock)).into_run();
    assert_eq!(ls_run.gate.outcome, GateOutcome::Allow);
    let debug = format!("{ls_run:?}");
    assert!(
        !debug.contains("OpTicket") && !debug.contains("ticket"),
        "the returned run must not carry anything a caller can hand to a computer"
    );
    assert_eq!(
        computer.logged(),
        vec!["run_command ls", "run_command done wakes 0"],
        "a finished run leaves nothing behind to ask the computer again",
    );
}
