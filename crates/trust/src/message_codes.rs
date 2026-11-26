pub const CODE_GATE_COMMAND_DENIED: &str = "gate-command-denied";
pub const CODE_GATE_PRIVATE_FILE: &str = "gate-private-file";
pub const CODE_GATE_NO_SANDBOX: &str = "gate-no-sandbox";
pub const CODE_GATE_BOT_MACHINE_OUTSIDE: &str = "gate-bot-machine-outside";
pub const CODE_GATE_OUTSIDE_WRITE_DENIED: &str = "gate-outside-write-denied";
pub const CODE_GATE_DENIED_BY_RULE: &str = "gate-denied-by-rule";
pub const CODE_GATE_DENIED: &str = "gate-denied";
pub const CODE_GATE_USER_REJECTED: &str = "gate-user-rejected";
pub const CODE_GATE_ASK_PENDING: &str = "gate-ask-pending";
pub const CODE_GATE_PASS_NOT_AUTHORIZED: &str = "gate-pass-not-authorized";
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

#[must_use]
pub fn all_codes() -> [&'static str; 20] {
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
        CODE_GATE_PASS_NOT_AUTHORIZED,
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_codes_are_unique_and_dash_separated() {
        let mut seen: Vec<&str> = Vec::new();
        for code in all_codes() {
            assert!(!seen.contains(&code), "duplicate code {code}");
            assert!(code.contains('-'), "code {code} has no dash");
            seen.push(code);
        }
        assert_eq!(seen.len(), 20);
    }
}
