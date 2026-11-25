use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::RunEvent;
#[cfg(test)]
use proptest as _;

pub mod always_list;
pub mod claim_levels;
pub mod command_key;
pub mod gate_engine;
pub mod gate_full;
pub mod safety_always;
pub mod safety_lock;
pub mod safety_n5;
pub mod trust_floor;

pub use always_list::{AddSource, AlwaysAddEvent, AlwaysEntry, AlwaysList, parse_untrusted_adds};
pub use claim_levels::{Claim, ClaimLevel, EvidenceKind, Link, Scope, Verification};
pub use command_key::{command_action_key, command_tokens};
pub use gate_engine::{GateLayer, LayerCall, Opinion, Outcome, precedence_decision};
pub use gate_full::{EngineInput, GateDecision, decide};
pub use safety_always::{ActionSpec, concrete_actions};
pub use safety_lock::{LockState, PolicyKey, parse_lock_value, parse_policy};
pub use safety_n5::{
    ActionClass, classes_of_click_label, classes_of_command, classes_of_connector_tool, word_hit,
};
pub use trust_floor::{FloorAction, FloorDeny, floor_check};

#[derive(Debug, Clone)]
pub struct Evidence {
    pub event: RunEvent,
}

pub type PassedSecret = Secret;

#[must_use]
pub fn status_needs_attention(status: TaskStatus) -> bool {
    anybot_domain::needs_attention(status)
}

#[cfg(test)]
mod always_list_tests;
#[cfg(test)]
mod claim_levels_tests;
#[cfg(test)]
mod gate_engine_tests;
#[cfg(test)]
mod gate_full_tests;
#[cfg(test)]
mod safety_always_tests;
#[cfg(test)]
mod safety_lock_tests;
#[cfg(test)]
mod safety_n5_tests;
