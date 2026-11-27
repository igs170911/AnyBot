use std::sync::Arc;

use anybot_foundation::{CallId, TaskId, TurnId};
use anybot_trust::{
    ActionContext, ActionRequest, GateConfig, RequestedAction, RunStep, run_action,
};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::test_view::TestView;

#[must_use]
pub fn fresh_clock() -> Arc<FakeClock> {
    FakeClock::new(1_000)
}

pub fn context(tool: &str, clock: &Arc<FakeClock>) -> ActionContext {
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

fn request(tool: &str, subject: RequestedAction, clock: &Arc<FakeClock>) -> ActionRequest {
    ActionRequest {
        context: context(tool, clock),
        subject,
    }
}

pub fn run(computer: &FakeComputer, req: &ActionRequest) -> RunStep {
    let clock = computer.clock();
    run_action(
        req,
        &anybot_trust::CallContext::new(
            CallId::generate(clock.as_ref()),
            TurnId::generate(clock.as_ref()),
            "c1",
        ),
        computer,
        &GateConfig::plain(),
        &TestView,
        clock.as_ref(),
    )
}

pub fn bot_computer(clock: &Arc<FakeClock>) -> FakeComputer {
    FakeComputerBuilder::default()
        .bot_machine()
        .clock(Arc::clone(clock))
        .build()
}

pub fn write_request(content: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "edit_file",
        RequestedAction::WriteFile {
            path: "notes.md".to_owned(),
            content: content.to_owned(),
        },
        clock,
    )
}

pub fn read_request(path: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "read_file",
        RequestedAction::ReadFile {
            path: path.to_owned(),
        },
        clock,
    )
}

pub fn command_request(line: &str, clock: &Arc<FakeClock>) -> ActionRequest {
    request(
        "shell",
        RequestedAction::Command {
            line: line.to_owned(),
            timeout_ms: 30_000,
        },
        clock,
    )
}
