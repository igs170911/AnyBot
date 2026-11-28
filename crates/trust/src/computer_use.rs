use crate::gate_full::ModeFlags;
use crate::message_codes_flow::{CODE_GATE_LOCAL_LOCKED, CODE_GATE_LOCAL_NEVER};
use crate::safety_lock::LockState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunHere {
    AlwaysAllow,
    AskEveryTime,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputerUse {
    pub usable_by_bot: bool,
    pub run_here: RunHere,
    pub this_machine: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPolicy {
    pub direct: LockState,
    pub local_execution: LockState,
}

impl LocalPolicy {
    #[must_use]
    pub fn unset() -> Self {
        LocalPolicy {
            direct: LockState::Unset,
            local_execution: LockState::Unset,
        }
    }

    #[must_use]
    pub fn locks_direct_button(&self) -> bool {
        self.direct == LockState::LockedOff || self.local_execution == LockState::LockedOff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchReason {
    BotNotAllowed,
    AskEveryTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepFour {
    Pass,
    Switch { reasons: Vec<SwitchReason> },
    Deny { code: &'static str },
}

#[must_use]
pub fn computer_use_step(
    computer: &ComputerUse,
    policy: &LocalPolicy,
    modes: &ModeFlags,
) -> StepFour {
    if computer.this_machine {
        if policy.local_execution == LockState::LockedOff {
            return StepFour::Deny {
                code: CODE_GATE_LOCAL_LOCKED,
            };
        }
        if computer.run_here == RunHere::Never {
            return StepFour::Deny {
                code: CODE_GATE_LOCAL_NEVER,
            };
        }
    }
    if direct_is_on(modes, policy) {
        return StepFour::Pass;
    }
    let mut reasons = Vec::new();
    if !computer.usable_by_bot {
        reasons.push(SwitchReason::BotNotAllowed);
    }
    if computer.this_machine && computer.run_here == RunHere::AskEveryTime {
        reasons.push(SwitchReason::AskEveryTime);
    }
    if reasons.is_empty() {
        StepFour::Pass
    } else {
        StepFour::Switch { reasons }
    }
}

#[must_use]
pub fn direct_is_on(modes: &ModeFlags, policy: &LocalPolicy) -> bool {
    modes.direct_mode && !modes.mdm_locks_direct && policy.direct != LockState::LockedOff
}

#[must_use]
pub fn direct_button_offered(modes: &ModeFlags, policy: &LocalPolicy) -> bool {
    !direct_is_on(modes, policy) && !policy.locks_direct_button() && !modes.mdm_locks_direct
}
