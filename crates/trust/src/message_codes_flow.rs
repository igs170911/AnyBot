pub const CODE_GATE_LOCAL_NEVER: &str = "gate-local-never";
pub const CODE_GATE_LOCAL_LOCKED: &str = "gate-local-locked";
pub const CODE_GATE_SWITCH_DECLINED_BOT: &str = "gate-switch-declined-bot";
pub const CODE_GATE_SWITCH_DECLINED_LOCAL: &str = "gate-switch-declined-local";
pub const CODE_CARD_SWITCH: &str = "card-switch";
pub const CODE_CARD_SWITCH_ACTION: &str = "card-switch-action";
pub const CODE_CARD_SWITCH_ALWAYS_BOT: &str = "card-switch-always-bot";
pub const CODE_CARD_SWITCH_ALWAYS_LOCAL: &str = "card-switch-always-local";

#[must_use]
pub fn flow_codes() -> Vec<&'static str> {
    vec![
        CODE_GATE_LOCAL_NEVER,
        CODE_GATE_LOCAL_LOCKED,
        CODE_GATE_SWITCH_DECLINED_BOT,
        CODE_GATE_SWITCH_DECLINED_LOCAL,
        CODE_CARD_SWITCH,
        CODE_CARD_SWITCH_ACTION,
        CODE_CARD_SWITCH_ALWAYS_BOT,
        CODE_CARD_SWITCH_ALWAYS_LOCAL,
    ]
}
