use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::RunEvent;
#[cfg(test)]
mod compilefail_tests {
    #[test]
    fn test_trust_compilefail_pass_api_is_closed_outside_trust() {
        trybuild::TestCases::new().compile_fail("tests/compilefail/*.rs");
    }
}

#[cfg(test)]
use proptest as _;

pub mod action;
pub mod always_list;
pub mod card_data;
pub mod claim_levels;
pub mod command_key;
pub mod command_target;
pub mod edit_rule;
pub mod evidence;
pub mod evidence_payload;
pub mod gate_engine;
pub mod gate_flow;
pub mod gate_full;
pub mod machine_rules;
#[cfg(test)]
mod machine_rules_tests;
pub mod message_codes;
pub mod op_ticket;
#[cfg(test)]
mod op_ticket_computer_tests;
pub mod pipeline;
pub mod pipeline_run;
pub mod pipeline_verify;
pub mod runner;
pub mod safety_always;
pub mod safety_lock;
pub mod safety_n5;
pub mod sandbox_deny;
pub mod sandbox_levels;
pub mod trust_floor;

pub use action::{ActionKey, ActionSubject, DEFAULT_COMMAND_TIMEOUT_MS, GateAction, PathView};
pub use always_list::{AddSource, AlwaysAddEvent, AlwaysEntry, AlwaysList, parse_untrusted_adds};
pub use card_data::{CardData, build_card};
pub use claim_levels::{Claim, ClaimLevel, EvidenceKind, Link, Scope, Verification};
pub use command_key::{command_action_key, command_tokens};
pub use command_target::{command_target_paths, has_expansion_token};
pub use edit_rule::{
    EditApplied, EditKind, EditRejection, apply_edit, line_count, occurrences, replace_once,
};
pub use evidence::ActionContext;
pub use gate_engine::{GateLayer, LayerCall, Opinion, Outcome, precedence_decision};
pub use gate_flow::{GateConfig, rule_code};
pub use gate_full::{EngineInput, GateDecision, decide};
pub use machine_rules::{MachineLevel, RuleVerdict, default_rule, inside_workdir};
pub use message_codes::{
    CODE_GATE_ASK_PENDING, CODE_GATE_BOT_MACHINE_OUTSIDE, CODE_GATE_COMMAND_DENIED,
    CODE_GATE_DENIED, CODE_GATE_DENIED_BY_RULE, CODE_GATE_NO_SANDBOX,
    CODE_GATE_OUTSIDE_WRITE_DENIED, CODE_GATE_PRIVATE_FILE, CODE_GATE_TICKET_MISMATCH,
    CODE_GATE_USER_REJECTED, CODE_TOOL_COMPUTER_UNREACHABLE, CODE_TOOL_EMPTY,
    CODE_TOOL_EXECUTION_FAILED, CODE_TOOL_OPERATION_FAILED, CODE_TURN_AWAITING_APPROVAL,
    CODE_TURN_COMPLETED, CODE_TURN_FAILED, CODE_TURN_NOT_EXECUTED, CODE_TURN_REJECTED,
    CODE_TURN_TOOL_FAILED, all_codes,
};
pub use op_ticket::{OpKind, OpTicket};
pub use pipeline::{ActionRequest, AskTicket, GateAnswer, answer_ask, run_action};
pub use pipeline_run::{ActionRun, ModelResultView, OutcomeText, RunStep, Summary};
pub use runner::{GatedComputer, OpRejection, OpRejectionKind};
pub use safety_always::{ActionSpec, concrete_actions};
pub use safety_lock::{LockState, PolicyKey, parse_lock_value, parse_policy};
pub use safety_n5::{
    ActionClass, classes_of_click_label, classes_of_command, classes_of_connector_tool, word_hit,
};
pub use sandbox_deny::{DENIED_COMMAND_KINDS, deny_token, normalize_command, split_segments};
pub use sandbox_levels::{CommandLevel, READ_ONLY_COMMANDS, command_level};
pub use trust_floor::{FloorAction, FloorDeny, floor_check, is_denied_command};

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
#[cfg(test)]
mod sandbox_deny_tests;
