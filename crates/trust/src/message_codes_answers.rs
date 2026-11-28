pub const CODE_GATE_ANSWER_NOT_OFFERED: &str = "gate-answer-not-offered";
pub const CODE_GATE_RULE_UNREADABLE: &str = "gate-rule-unreadable";
pub const CODE_GATE_REJECT_REPEAT: &str = "gate-reject-repeat";

#[must_use]
pub fn answer_codes() -> Vec<&'static str> {
    vec![
        CODE_GATE_ANSWER_NOT_OFFERED,
        CODE_GATE_RULE_UNREADABLE,
        CODE_GATE_REJECT_REPEAT,
    ]
}
