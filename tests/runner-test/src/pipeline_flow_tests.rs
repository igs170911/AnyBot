use std::sync::Arc;

use anybot_protocol::{GateOutcome, JudgementSource};
use anybot_trust::GatedComputer;
use anybot_trust::{
    ActionSubject, GateAnswer, GateConfig, PathView, RunStep, answer_ask, run_action,
};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::fake_scripts::CommandScript;
use crate::ops_tests::{command_request, fresh_clock, run};
use crate::spy_computer::SpyComputer;
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
fn test_the_computer_rejects_a_pass_issued_for_a_different_action() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("ls", CommandScript::ok("files"))
        .build();
    let ls_run = run(&computer, &command_request("ls", &clock)).into_run();
    let Some(pass) = ls_run.pass else {
        panic!("an executed run must carry the pass it used");
    };
    let result = computer.run_command(&pass, "pwd", 1000);
    assert!(
        result.is_err(),
        "a pass issued for ls must be rejected by the computer for pwd"
    );
}

#[test]
fn test_a_pass_for_one_command_never_authorizes_another_command() {
    let clock = fresh_clock();
    let inner = FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(&clock))
        .script("ls", CommandScript::ok("files"))
        .script("pwd", CommandScript::ok("/workspace"))
        .build();
    let spy = SpyComputer::new(inner);
    let ls_run = run_action(
        &command_request("ls", &clock),
        &spy,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    )
    .into_run();
    assert_eq!(ls_run.gate.outcome, GateOutcome::Allow);
    assert_eq!(spy.seen(), vec![false]);
    let pwd_run = run_action(
        &command_request("pwd", &clock),
        &spy,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    )
    .into_run();
    assert_eq!(pwd_run.gate.outcome, GateOutcome::Allow);
    assert_eq!(spy.seen(), vec![false, true]);
}
