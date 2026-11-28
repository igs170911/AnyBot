use anybot_domain::TaskStatus;
use anybot_foundation::Secret;
use anybot_protocol::RunEvent;
#[cfg(test)]
mod compilefail_tests {
    #[test]
    fn test_trust_pass_api_is_closed_outside_trust_compile_fail() {
        trybuild::TestCases::new().compile_fail("tests/compilefail/*.rs");
    }
}

#[cfg(test)]
use proptest as _;

pub mod action;
pub mod always_list;
pub mod card_buttons;
pub mod card_data;
mod card_reason;
pub mod card_view;
#[cfg(test)]
mod card_view_tests;
pub mod claim_levels;
pub mod claims_check;
#[cfg(test)]
mod claims_check_tests;
mod claims_links;
pub mod claims_phrases;
mod claims_text;
pub mod command_key;
pub mod command_lex;
pub mod command_read;
pub mod command_target;
pub mod command_wrappers;
pub mod computer_use;
pub mod edit_rule;
pub mod evidence;
pub mod evidence_payload;
pub mod gate_engine;
pub mod gate_flow;
mod gate_flow_params;
pub mod gate_full;
pub mod gate_rules;
pub mod machine_rules;
#[cfg(test)]
mod machine_rules_tests;
pub mod message_codes;
pub mod message_codes_answers;
pub mod message_codes_flow;
pub mod message_codes_report;
pub mod op_ticket;
#[cfg(test)]
mod op_ticket_computer_tests;
mod path_resolve;
pub mod pattern;
mod pattern_glob;
mod pattern_web;
pub mod pipeline;
mod pipeline_ask;
mod pipeline_denied;
pub mod pipeline_exec;
mod pipeline_gate;
pub mod pipeline_run;
mod pipeline_switch;
pub mod pipeline_verify;
pub mod private_paths;
#[cfg(test)]
mod private_paths_tests;
pub mod report;
mod report_attention;
mod report_claim_checks;
pub mod report_claims;
mod report_due;
mod report_extract;
mod report_progress;
mod report_sentences;
pub mod runner;
pub mod safety_always;
pub mod safety_lock;
pub mod safety_n5;
pub mod sandbox_deny;
pub mod sandbox_levels;
pub mod sandbox_peel;
mod six_class;
pub mod trust_floor;
mod wrapper_flags;

pub use action::{ActionKey, ActionSubject, DEFAULT_COMMAND_TIMEOUT_MS, GateAction, PathView};
pub use always_list::{AddSource, AlwaysAddEvent, AlwaysEntry, AlwaysList, parse_untrusted_adds};
pub use card_buttons::{AskOrigin, button_set};
pub use card_data::{CardData, build_card};
pub use card_view::{CardView, CardViewError, card_view};
pub use claim_levels::{Claim, ClaimLevel, EvidenceKind, Link, Scope, Verification};
pub use claims_check::{ClaimVerdict, StepFact, TurnFacts, check_reply, is_send_tool};
pub use claims_phrases::{DONE_PHRASES, EN_DONE_PHRASES, EN_SEND_PHRASES, SEND_PHRASES};
pub use command_key::{command_action_key, command_tokens};
pub use command_target::{DeleteBase, DeleteTarget, command_targets, has_expansion_token};
pub use computer_use::{
    ComputerUse, LocalPolicy, RunHere, StepFour, SwitchReason, computer_use_step,
    direct_button_offered, direct_is_on,
};
pub use edit_rule::{
    EDIT_MULTIPLE_REASON, EDIT_NOT_FOUND_REASON, EditApplied, EditKind, EditRejection, apply_edit,
    line_count, occurrences, replace_once,
};
pub use evidence::ActionContext;
pub use gate_engine::{GateLayer, LayerCall, Opinion, Outcome, precedence_decision};
pub use gate_flow::{AdminRules, GateConfig, GateVerdict, RuleEntry, UserRules, rule_code};
pub use gate_full::{ModeFlags, StyleFlags};
pub use machine_rules::{
    MachineLevel, RuleVerdict, default_rule, default_rule_with_always, inside_workdir,
};
pub use message_codes::{
    CODE_ALWAYS_READ_OUTSIDE, CODE_ALWAYS_READ_WORKSPACE, CODE_ALWAYS_RUN_COMMAND,
    CODE_ALWAYS_RUN_COMMAND_BOT_COMPUTER, CODE_ALWAYS_WRITE_WORKSPACE, CODE_CARD_ADMIN_RULE,
    CODE_CARD_ASK_RULE, CODE_CARD_EDIT_FILE, CODE_CARD_EDIT_OUTSIDE, CODE_CARD_FLOOR_NO_SANDBOX,
    CODE_CARD_FLOOR_RULE, CODE_CARD_HIDDEN_SUBCOMMAND, CODE_CARD_INTERACTIVE_SHELL,
    CODE_CARD_NOTE_EVERY_TIME, CODE_CARD_NOTE_NO_SANDBOX_PRIVATE, CODE_CARD_READ_FILE,
    CODE_CARD_READ_OUTSIDE, CODE_CARD_RUN_COMMAND, CODE_CARD_UNPARSEABLE, CODE_CARD_USER_HOOK,
    CODE_CLAIM_CHAT_COUNT, CODE_CLAIM_CHAT_DEAD_LINK, CODE_CLAIM_CHAT_NO_FILE,
    CODE_CLAIM_CHAT_STEP_FAILED, CODE_CLAIM_COUNT_MISMATCH, CODE_CLAIM_KIND_MISMATCH,
    CODE_CLAIM_NO_EVIDENCE, CODE_CLAIM_NOT_IN_TASK, CODE_CLAIM_RECIPIENT_MISMATCH,
    CODE_CLAIM_STEP_FAILED, CODE_GATE_ASK_DECLINED, CODE_GATE_ASK_PENDING,
    CODE_GATE_BOT_MACHINE_OUTSIDE, CODE_GATE_COMMAND_DENIED, CODE_GATE_DENIED,
    CODE_GATE_DENIED_BY_RULE, CODE_GATE_NO_SANDBOX, CODE_GATE_OUTSIDE_WRITE_DENIED,
    CODE_GATE_PRIVATE_FILE, CODE_GATE_TICKET_MISMATCH, CODE_GATE_USER_REJECTED,
    CODE_TOOL_COMPUTER_UNREACHABLE, CODE_TOOL_EMPTY, CODE_TOOL_EXECUTION_FAILED,
    CODE_TOOL_OPERATION_FAILED, CODE_TOOL_UNREADABLE_SHAPE, CODE_TURN_AWAITING_APPROVAL,
    CODE_TURN_COMPLETED, CODE_TURN_FAILED, CODE_TURN_NOT_EXECUTED, CODE_TURN_REJECTED,
    CODE_TURN_TOOL_FAILED, all_codes,
};
pub use message_codes::{CODE_REPORT_SUMMARY_MISSING, CODE_TURN_ANSWER_EMPTY};
pub use message_codes_answers::{
    CODE_GATE_ANSWER_NOT_OFFERED, CODE_GATE_REJECT_REPEAT, CODE_GATE_RULE_UNREADABLE,
};
pub use message_codes_flow::{
    CODE_CARD_SWITCH, CODE_CARD_SWITCH_ACTION, CODE_CARD_SWITCH_ALWAYS_BOT,
    CODE_CARD_SWITCH_ALWAYS_LOCAL, CODE_GATE_LOCAL_LOCKED, CODE_GATE_LOCAL_NEVER,
    CODE_GATE_SWITCH_DECLINED_BOT, CODE_GATE_SWITCH_DECLINED_LOCAL,
};
pub use message_codes_report::{CODE_REPORT_NEXT_RAISE_LIMIT, CODE_REPORT_STUCK_OVER_BUDGET};
pub use op_ticket::{OpKind, OpTicket};
pub use pipeline::{ActionRequest, GateAnswer, RequestedAction};
pub use pipeline_ask::{AskTicket, CallContext, answer_ask};
pub use pipeline_denied::{
    StepDenial, rejected_just_now_denial, step_denied_run, step_four_denial, unreadable_denial,
};
pub use pipeline_gate::{GateStep, ask_declined_run, exec_step, gate_step, run_action};
pub use pipeline_run::{ActionRun, ModelResultView, OutcomeText, RunStep, Summary};
pub use pipeline_switch::{SWITCH_RULE, SwitchInfo, switch_ticket};
pub use private_paths::is_private_path;
pub use report::{ReportFail, delivery_report, delivery_report_for_task, report_summary};
pub use report_claims::{VerifiedRef, count_levels, grade_reply};
pub use report_due::{ReportDue, report_due};
pub use report_progress::{budget_item_name, progress_of};
pub use runner::{GatedComputer, OpRejection, OpRejectionKind};
pub use safety_always::{ActionSpec, concrete_actions};
pub use safety_lock::{LockState, PolicyKey, parse_lock_value, parse_policy};
pub use safety_n5::{
    ActionClass, classes_of_click_label, classes_of_command, classes_of_connector_tool, word_hit,
};
pub use sandbox_deny::{DENIED_COMMAND_KINDS, deny_token, normalize_command, split_segments};
pub use sandbox_levels::{CommandLevel, READ_ONLY_COMMANDS, command_level};
pub use sandbox_peel::{
    PEEL_WRAPPERS, PRIVILEGE_WRAPPERS, bare_name, peel_segment, unreadable_reason, visible_segments,
};
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
mod command_shape_tests;
#[cfg(test)]
mod computer_use_tests;
#[cfg(test)]
mod gate_engine_tests;
#[cfg(test)]
mod gate_flow_tests;
#[cfg(test)]
mod gate_full_tests;
#[cfg(test)]
mod pattern_tests;
#[cfg(test)]
mod report_claims_prop_tests;
#[cfg(test)]
mod report_claims_tests;
#[cfg(test)]
mod report_due_tests;
#[cfg(test)]
mod safety_always_tests;
#[cfg(test)]
mod safety_lock_tests;
#[cfg(test)]
mod safety_n5_tests;
#[cfg(test)]
mod sandbox_deny_tests;
#[cfg(test)]
mod sandbox_peel_tests;
