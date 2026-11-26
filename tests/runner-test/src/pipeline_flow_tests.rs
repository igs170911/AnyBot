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

#[derive(Debug, Default)]
pub(crate) struct PassSpy {
    seen: Vec<bool>,
}

impl PassSpy {
    fn note(&mut self, authorized_other: bool) {
        self.seen.push(authorized_other);
    }
}

use std::sync::Mutex;

use anybot_protocol::ActualResult;
use anybot_trust::{ActionKey, OpRejection, Pass};

pub(crate) struct SpyComputer {
    inner: FakeComputer,
    spy: Mutex<PassSpy>,
}

impl SpyComputer {
    #[must_use]
    pub(crate) fn new(inner: FakeComputer) -> Self {
        SpyComputer {
            inner,
            spy: Mutex::new(PassSpy::default()),
        }
    }

    fn watch(&self, pass: &Pass) {
        let other = ActionKey::command("shell", "pwd");
        let mut spy = self
            .spy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        spy.note(pass.authorizes(&other));
    }

    #[must_use]
    pub(crate) fn seen(&self) -> Vec<bool> {
        self.spy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .seen
            .clone()
    }
}

impl GatedComputer for SpyComputer {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn declared_caps(&self) -> anybot_protocol::ComputerCaps {
        self.inner.declared_caps()
    }

    fn capabilities(&self, pass: &Pass) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.capabilities(pass)
    }

    fn resolve_target(&self, path: &str) -> Option<String> {
        self.inner.resolve_target(path)
    }

    fn path_info(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.path_info(pass, path)
    }

    fn read_file(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.read_file(pass, path)
    }

    fn write_file(
        &self,
        pass: &Pass,
        path: &str,
        content: &str,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.write_file(pass, path, content)
    }

    fn edit_file(
        &self,
        pass: &Pass,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.edit_file(pass, path, old_text, new_text)
    }

    fn run_command(
        &self,
        pass: &Pass,
        command: &str,
        timeout_ms: u64,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.run_command(pass, command, timeout_ms)
    }
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
