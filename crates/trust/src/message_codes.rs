pub const CODE_GATE_COMMAND_DENIED: &str = "gate-command-denied";
pub const CODE_GATE_PRIVATE_FILE: &str = "gate-private-file";
pub const CODE_GATE_NO_SANDBOX: &str = "gate-no-sandbox";
pub const CODE_GATE_BOT_MACHINE_OUTSIDE: &str = "gate-bot-machine-outside";
pub const CODE_GATE_OUTSIDE_WRITE_DENIED: &str = "gate-outside-write-denied";
pub const CODE_GATE_DENIED_BY_RULE: &str = "gate-denied-by-rule";
pub const CODE_GATE_DENIED: &str = "gate-denied";
pub const CODE_GATE_USER_REJECTED: &str = "gate-user-rejected";
pub const CODE_GATE_ASK_PENDING: &str = "gate-ask-pending";
pub const CODE_GATE_ASK_DECLINED: &str = "gate-ask-declined";
pub const CODE_REPORT_SUMMARY_MISSING: &str = "report-summary-missing";
pub const CODE_TURN_ANSWER_EMPTY: &str = "turn-answer-empty";
pub const CODE_GATE_ASK_UNAVAILABLE: &str = "gate-ask-unavailable";
pub const CODE_GATE_TICKET_MISMATCH: &str = "gate-ticket-mismatch";
pub const CODE_GATE_ASK_COMPUTER_MISMATCH: &str = "gate-ask-computer-mismatch";
pub const CODE_TURN_COMPLETED: &str = "turn-completed";
pub const CODE_TURN_FAILED: &str = "turn-failed";
pub const CODE_TURN_TOOL_FAILED: &str = "turn-tool-failed";
pub const CODE_TURN_REJECTED: &str = "turn-rejected";
pub const CODE_TURN_NOT_EXECUTED: &str = "turn-not-executed";
pub const CODE_TURN_AWAITING_APPROVAL: &str = "turn-awaiting-approval";
pub const CODE_TOOL_EMPTY: &str = "tool-empty";
pub const CODE_TOOL_OPERATION_FAILED: &str = "tool-operation-failed";
pub const CODE_TOOL_COMPUTER_UNREACHABLE: &str = "tool-computer-unreachable";
pub const CODE_TOOL_EXECUTION_FAILED: &str = "tool-execution-failed";
pub const CODE_TOOL_UNREADABLE_SHAPE: &str = "tool-unreadable-shape";
pub const CODE_CARD_EDIT_FILE: &str = "card-edit-file";
pub const CODE_CARD_RUN_COMMAND: &str = "card-run-command";
pub const CODE_CARD_READ_OUTSIDE: &str = "card-read-outside";
pub const CODE_CARD_READ_FILE: &str = "card-read-file";
pub const CODE_CARD_EDIT_OUTSIDE: &str = "card-edit-outside";
pub const CODE_CARD_UNPARSEABLE: &str = "card-unparseable";
pub const CODE_CARD_HIDDEN_SUBCOMMAND: &str = "card-hidden-subcommand";
pub const CODE_CARD_INTERACTIVE_SHELL: &str = "card-interactive-shell";
pub const CODE_CARD_ASK_RULE: &str = "card-ask-rule";
pub const CODE_CARD_USER_HOOK: &str = "card-user-hook";
pub const CODE_CARD_ADMIN_RULE: &str = "card-admin-rule";
pub const CODE_CARD_FLOOR_RULE: &str = "card-floor-rule";
pub const CODE_CARD_FLOOR_NO_SANDBOX: &str = "card-floor-no-sandbox";
pub const CODE_CARD_NOTE_EVERY_TIME: &str = "card-note-every-time";
pub const CODE_CARD_NOTE_NO_SANDBOX_PRIVATE: &str = "card-note-no-sandbox-private";
pub const CODE_ALWAYS_RUN_COMMAND: &str = "always-run-command";
pub const CODE_ALWAYS_RUN_COMMAND_BOT_COMPUTER: &str = "always-run-command-bot-computer";
pub const CODE_ALWAYS_WRITE_WORKSPACE: &str = "always-write-workspace";
pub const CODE_ALWAYS_READ_OUTSIDE: &str = "always-read-outside";
pub const CODE_ALWAYS_READ_WORKSPACE: &str = "always-read-workspace";
pub const CODE_CLAIM_CHAT_STEP_FAILED: &str = "claim-chat-step-failed";
pub const CODE_CLAIM_CHAT_NO_FILE: &str = "claim-chat-no-file";
pub const CODE_CLAIM_CHAT_COUNT: &str = "claim-chat-count";
pub const CODE_CLAIM_CHAT_DEAD_LINK: &str = "claim-chat-dead-link";
pub const CODE_CLAIM_NO_EVIDENCE: &str = "claim-no-evidence";
pub const CODE_CLAIM_STEP_FAILED: &str = "claim-step-failed";
pub const CODE_CLAIM_KIND_MISMATCH: &str = "claim-kind-mismatch";
pub const CODE_CLAIM_COUNT_MISMATCH: &str = "claim-count-mismatch";
pub const CODE_CLAIM_RECIPIENT_MISMATCH: &str = "claim-recipient-mismatch";
pub const CODE_CLAIM_NOT_IN_TASK: &str = "claim-not-in-task";

#[must_use]
pub fn base_codes() -> [&'static str; 56] {
    [
        CODE_GATE_COMMAND_DENIED,
        CODE_GATE_PRIVATE_FILE,
        CODE_GATE_NO_SANDBOX,
        CODE_GATE_BOT_MACHINE_OUTSIDE,
        CODE_GATE_OUTSIDE_WRITE_DENIED,
        CODE_GATE_DENIED_BY_RULE,
        CODE_GATE_DENIED,
        CODE_GATE_USER_REJECTED,
        CODE_GATE_ASK_PENDING,
        CODE_GATE_ASK_DECLINED,
        CODE_GATE_ASK_UNAVAILABLE,
        CODE_REPORT_SUMMARY_MISSING,
        CODE_TURN_ANSWER_EMPTY,
        CODE_GATE_ASK_COMPUTER_MISMATCH,
        CODE_GATE_TICKET_MISMATCH,
        CODE_TURN_COMPLETED,
        CODE_TURN_FAILED,
        CODE_TURN_TOOL_FAILED,
        CODE_TURN_REJECTED,
        CODE_TURN_NOT_EXECUTED,
        CODE_TURN_AWAITING_APPROVAL,
        CODE_TOOL_EMPTY,
        CODE_TOOL_OPERATION_FAILED,
        CODE_TOOL_COMPUTER_UNREACHABLE,
        CODE_TOOL_EXECUTION_FAILED,
        CODE_TOOL_UNREADABLE_SHAPE,
        CODE_CARD_EDIT_FILE,
        CODE_CARD_RUN_COMMAND,
        CODE_CARD_READ_OUTSIDE,
        CODE_CARD_READ_FILE,
        CODE_CARD_EDIT_OUTSIDE,
        CODE_CARD_UNPARSEABLE,
        CODE_CARD_HIDDEN_SUBCOMMAND,
        CODE_CARD_INTERACTIVE_SHELL,
        CODE_CARD_ASK_RULE,
        CODE_CARD_USER_HOOK,
        CODE_CARD_ADMIN_RULE,
        CODE_CARD_FLOOR_RULE,
        CODE_CARD_FLOOR_NO_SANDBOX,
        CODE_CARD_NOTE_EVERY_TIME,
        CODE_CARD_NOTE_NO_SANDBOX_PRIVATE,
        CODE_ALWAYS_RUN_COMMAND,
        CODE_ALWAYS_RUN_COMMAND_BOT_COMPUTER,
        CODE_ALWAYS_WRITE_WORKSPACE,
        CODE_ALWAYS_READ_OUTSIDE,
        CODE_ALWAYS_READ_WORKSPACE,
        CODE_CLAIM_CHAT_STEP_FAILED,
        CODE_CLAIM_CHAT_NO_FILE,
        CODE_CLAIM_CHAT_COUNT,
        CODE_CLAIM_CHAT_DEAD_LINK,
        CODE_CLAIM_NO_EVIDENCE,
        CODE_CLAIM_STEP_FAILED,
        CODE_CLAIM_KIND_MISMATCH,
        CODE_CLAIM_COUNT_MISMATCH,
        CODE_CLAIM_RECIPIENT_MISMATCH,
        CODE_CLAIM_NOT_IN_TASK,
    ]
}

#[must_use]
pub fn all_codes() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = base_codes().to_vec();
    all.extend(crate::message_codes_flow::flow_codes());
    all.extend(crate::message_codes_answers::answer_codes());
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_codes_answers::answer_codes;
    use crate::message_codes_flow::flow_codes;

    #[test]
    fn test_message_codes_are_unique_and_dash_separated() {
        let mut seen: Vec<&str> = Vec::new();
        for code in all_codes() {
            assert!(!seen.contains(&code), "duplicate code {code}");
            assert!(code.contains('-'), "code {code} has no dash");
            seen.push(code);
        }
        assert_eq!(
            seen.len(),
            base_codes().len() + flow_codes().len() + answer_codes().len()
        );
    }
}
