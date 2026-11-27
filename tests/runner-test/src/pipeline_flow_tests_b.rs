use std::sync::Arc;

use anybot_foundation::CallId;
use anybot_protocol::{EvidencePayload, GateOutcome};
use anybot_trust::{GateConfig, RunStep, run_action};

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_scripts::CommandScript;
use crate::ops_support::{command_request, fresh_clock, run};
use crate::test_view::TestView;

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

#[test]
fn test_a_wrapper_with_the_real_name_and_a_root_workdir_cannot_spend_the_ticket() {
    let clock = fresh_clock();
    let computer = FakeComputerBuilder::default()
        .bot_machine()
        .named("bot-1")
        .clock(Arc::clone(&clock))
        .script("pwd", CommandScript::ok("/workspace"))
        .build();
    computer.shift_declaration_before_the_operation();
    let step = run_action(
        &command_request("pwd", &clock),
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
    let RunStep::Executed(run) = step else {
        panic!("the gate issued a ticket before the declaration changed");
    };
    assert_eq!(computer.logged().len(), 0, "{run:?}");
    assert!(
        run.evidence.iter().any(|item| {
            matches!(
                &item.payload,
                EvidencePayload::Rejected { reason, .. } if reason.code == "gate-ticket-mismatch"
            )
        }),
        "{run:?}"
    );
}
