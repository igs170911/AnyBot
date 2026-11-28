use crate::computer_use::{
    ComputerUse, LocalPolicy, RunHere, StepFour, SwitchReason, computer_use_step,
    direct_button_offered,
};
use crate::gate_full::ModeFlags;
use crate::message_codes_flow::{CODE_GATE_LOCAL_LOCKED, CODE_GATE_LOCAL_NEVER};
use crate::safety_lock::LockState;

fn modes(direct: bool) -> ModeFlags {
    ModeFlags {
        ask_every_time: false,
        yolo: false,
        direct_mode: direct,
        mdm_locks_direct: false,
    }
}

fn computer(usable: bool, run_here: RunHere) -> ComputerUse {
    ComputerUse {
        usable_by_bot: usable,
        run_here,
        this_machine: true,
    }
}

#[test]
fn computer_use_step_table() {
    let open = LocalPolicy::unset();
    assert_eq!(
        computer_use_step(&computer(false, RunHere::AlwaysAllow), &open, &modes(false)),
        StepFour::Switch {
            reasons: vec![SwitchReason::BotNotAllowed]
        }
    );
    let locked = LocalPolicy {
        direct: LockState::Unset,
        local_execution: LockState::LockedOff,
    };
    assert_eq!(
        computer_use_step(
            &computer(true, RunHere::AlwaysAllow),
            &locked,
            &modes(false)
        ),
        StepFour::Deny {
            code: CODE_GATE_LOCAL_LOCKED
        }
    );
    assert_eq!(
        computer_use_step(&computer(true, RunHere::Never), &open, &modes(false)),
        StepFour::Deny {
            code: CODE_GATE_LOCAL_NEVER
        }
    );
    assert_eq!(
        computer_use_step(&computer(true, RunHere::Never), &open, &modes(true)),
        StepFour::Deny {
            code: CODE_GATE_LOCAL_NEVER
        }
    );
    assert_eq!(
        computer_use_step(&computer(false, RunHere::AskEveryTime), &open, &modes(true)),
        StepFour::Pass
    );
    assert_eq!(
        computer_use_step(&computer(true, RunHere::AlwaysAllow), &open, &modes(false)),
        StepFour::Pass
    );
}

#[test]
fn every_time_and_unticked_give_both_reasons() {
    let step = computer_use_step(
        &computer(false, RunHere::AskEveryTime),
        &LocalPolicy::unset(),
        &modes(false),
    );
    assert_eq!(
        step,
        StepFour::Switch {
            reasons: vec![SwitchReason::BotNotAllowed, SwitchReason::AskEveryTime]
        }
    );
}

#[test]
fn locked_direct_hides_the_direct_button() {
    let locked = LocalPolicy {
        direct: LockState::LockedOff,
        local_execution: LockState::Unset,
    };
    assert!(!direct_button_offered(&modes(false), &locked));
    assert!(direct_button_offered(&modes(false), &LocalPolicy::unset()));
    assert!(!direct_button_offered(&modes(true), &LocalPolicy::unset()));
}
