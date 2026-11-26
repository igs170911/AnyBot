use crate::gate_engine::{GateLayer, LayerCall, Opinion, Outcome, precedence_decision};
use crate::trust_floor::{FloorAction, floor_check};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeFlags {
    pub six_class: bool,
    pub ask_every_time: bool,
    pub yolo: bool,
    pub direct_mode: bool,
    pub mdm_locks_direct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleFlags {
    pub ask_style_hit: bool,
    pub allow_style_hit: bool,
    pub always_hit: bool,
    pub session_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EngineInput {
    pub action: FloorAction,
    pub modes: ModeFlags,
    pub styles: StyleFlags,
    pub admin_hook: Opinion,
    pub user_hook: Opinion,
    pub default_rule: Opinion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateDecision {
    pub outcome: Outcome,
    pub layer: GateLayer,
    pub rule: String,
}

fn unreadable_command(action: &FloorAction) -> bool {
    matches!(
        action,
        FloorAction::Command { line, .. } if crate::sandbox_peel::unreadable_reason(line).is_some()
    )
}

fn floor_call(input: &EngineInput, floor_hit: bool) -> LayerCall {
    let unreadable = unreadable_command(&input.action);
    LayerCall {
        layer: GateLayer::Floor,
        deny: (floor_hit && !unreadable) || input.modes.ask_every_time,
        ask: unreadable,
        allow: false,
    }
}

fn admin_call(input: &EngineInput) -> LayerCall {
    LayerCall {
        layer: GateLayer::Admin,
        deny: input.modes.mdm_locks_direct || input.admin_hook == Opinion::Deny,
        ask: input.admin_hook == Opinion::Ask,
        allow: input.admin_hook == Opinion::Allow,
    }
}

fn user_call(input: &EngineInput) -> LayerCall {
    let can_ask = !input.modes.ask_every_time;
    let list_allow = input.styles.always_hit || input.styles.session_allowed;
    let style_allow = input.styles.allow_style_hit && !input.modes.six_class;
    let direct_allow = input.modes.direct_mode && !input.modes.six_class;
    let hook_allow = input.user_hook == Opinion::Allow && !input.modes.six_class;
    LayerCall {
        layer: GateLayer::User,
        deny: input.user_hook == Opinion::Deny || input.modes.ask_every_time,
        ask: (input.user_hook == Opinion::Ask && can_ask) || input.styles.ask_style_hit,
        allow: list_allow || style_allow || direct_allow || hook_allow,
    }
}

fn default_call(input: &EngineInput) -> LayerCall {
    LayerCall {
        layer: GateLayer::Default,
        deny: input.default_rule == Opinion::Deny,
        ask: input.default_rule == Opinion::Ask,
        allow: input.default_rule == Opinion::Allow,
    }
}

#[must_use]
pub(crate) fn decide(input: &EngineInput) -> GateDecision {
    let floor_hit = floor_check(&input.action).is_some();
    let calls = [
        floor_call(input, floor_hit),
        admin_call(input),
        user_call(input),
        default_call(input),
    ];
    let (layer, outcome) =
        precedence_decision(&calls).unwrap_or((GateLayer::Default, Outcome::Ask));
    let outcome = relax_outcome(input, layer, outcome);
    GateDecision {
        outcome,
        layer,
        rule: format!("{layer}"),
    }
}

fn relax_outcome(input: &EngineInput, layer: GateLayer, outcome: Outcome) -> Outcome {
    let unreadable = unreadable_command(&input.action);
    match outcome {
        Outcome::Ask if unreadable => Outcome::Ask,
        Outcome::Ask if input.modes.yolo && !input.modes.ask_every_time => Outcome::Allow,
        Outcome::Ask if input.modes.direct_mode && layer == GateLayer::Default => Outcome::Ask,
        Outcome::Ask => Outcome::Ask,
        Outcome::Deny => Outcome::Deny,
        Outcome::Allow => Outcome::Allow,
    }
}
