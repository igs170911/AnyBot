use crate::action::{ActionKey, ActionSubject, GateAction};
use crate::always_list::AlwaysList;
use crate::gate_engine::{GateLayer, Opinion};
use crate::gate_flow_params::{decision_rule, floor_denied_token, gate_outcome, verdict_params};
use crate::gate_full::{EngineInput, ModeFlags, decide};
use crate::gate_rules::{hook_opinion, style_hits};
use crate::machine_rules::default_rule;
use crate::op_ticket::OpTicket;
use crate::trust_floor::{FloorAction, floor_check};
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    ComputerCaps, ComputerOwner, GateNo, GateOutcome, GateRecord, JudgementSource, RuleLayer,
};

pub use crate::gate_flow_params::rule_code;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEntry {
    pub key: ActionKey,
    pub opinion: Opinion,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserRules {
    pub always: AlwaysList,
    pub session: Vec<ActionKey>,
    pub ask_style: Vec<ActionKey>,
    pub allow_style: Vec<ActionKey>,
    pub hook: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdminRules {
    pub hook: Vec<RuleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfig {
    pub modes: ModeFlags,
    pub user: UserRules,
    pub admin: AdminRules,
    pub anybot_dir: String,
}

impl GateConfig {
    #[must_use]
    pub fn plain() -> Self {
        GateConfig {
            modes: ModeFlags {
                ask_every_time: false,
                yolo: false,
                direct_mode: false,
                mdm_locks_direct: false,
            },
            user: UserRules::default(),
            admin: AdminRules::default(),
            anybot_dir: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateVerdict {
    pub(crate) outcome: GateOutcome,
    pub(crate) layer: GateLayer,
    pub(crate) rule: String,
    pub(crate) read_only: bool,
    pub(crate) params: Vec<(String, String)>,
}

impl GateVerdict {
    #[must_use]
    pub(crate) fn gate_no(&self) -> GateNo {
        GateNo(match self.layer {
            GateLayer::Floor => 1,
            GateLayer::Admin => 2,
            GateLayer::User => 3,
            GateLayer::Default => 4,
        })
    }

    #[must_use]
    pub(crate) fn record(&self) -> GateRecord {
        GateRecord::new(
            self.gate_no(),
            rule_layer(self.layer),
            self.rule.clone(),
            JudgementSource::Rule,
            self.outcome,
            self.read_only,
        )
    }

    #[must_use]
    pub(crate) fn ticket(
        &self,
        action: &GateAction,
        computer: &str,
        caps: &ComputerCaps,
        issued_at: TimestampMs,
    ) -> Option<OpTicket> {
        if self.outcome != GateOutcome::Allow {
            return None;
        }
        Some(OpTicket::issue(
            &action.subject,
            action.key(),
            self.record(),
            computer,
            caps,
            &action.workdir,
            issued_at,
        ))
    }
}

fn rule_layer(layer: GateLayer) -> RuleLayer {
    match layer {
        GateLayer::Floor => RuleLayer::Floor,
        GateLayer::Admin => RuleLayer::Admin,
        GateLayer::User => RuleLayer::User,
        GateLayer::Default => RuleLayer::Default,
    }
}

#[must_use]
pub fn floor_view(action: &GateAction, caps: &ComputerCaps) -> FloorAction {
    floor_view_with_dir(action, caps, "")
}

#[must_use]
pub(crate) fn floor_view_with_dir(
    action: &GateAction,
    caps: &ComputerCaps,
    anybot_dir: &str,
) -> FloorAction {
    let bot_machine = caps.owner == ComputerOwner::Bot;
    match &action.subject {
        ActionSubject::Command {
            line,
            timeout_ms: _,
        } => FloorAction::Command {
            line: line.clone(),
            bot_machine,
            has_sandbox: caps.command_sandbox.yes(),
        },
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, content: _ }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => file_floor_action(path, caps, anybot_dir),
        ActionSubject::Capabilities => FloorAction::Capabilities,
    }
}

fn file_floor_action(
    path: &crate::action::PathView,
    caps: &ComputerCaps,
    anybot_dir: &str,
) -> FloorAction {
    if !path.is_resolvable() {
        return FloorAction::UnresolvedPath;
    }
    let Some(canonical) = path.resolved_text() else {
        return FloorAction::UnresolvedPath;
    };
    FloorAction::FilePath {
        path: canonical.to_owned(),
        caps: caps.clone(),
        anybot_dir: anybot_dir.to_owned(),
    }
}

#[must_use]
pub(crate) fn judge(
    action: &GateAction,
    caps: &ComputerCaps,
    config: &GateConfig,
    computer: &str,
) -> GateVerdict {
    let verdict = default_rule(action, caps);
    let floor_action = floor_view_with_dir(action, caps, &config.anybot_dir);
    let floor_hit = floor_check(&floor_action);
    let input = EngineInput {
        action: floor_action,
        modes: config.modes.clone(),
        six_class: crate::six_class::holds_a_class(action, caps.owner == ComputerOwner::Bot),
        styles: style_hits(action, computer, &config.user),
        admin_hook: hook_opinion(&config.admin.hook, &action.key()),
        user_hook: hook_opinion(&config.user.hook, &action.key()),
        default_rule: verdict.opinion(),
    };

    let decision = decide(&input);
    let rule = decision_rule(decision.layer, &verdict, floor_hit);
    let bot_machine = caps.owner == ComputerOwner::Bot;
    let denied_token = floor_denied_token(action, bot_machine);
    let params = verdict_params(decision.layer, floor_hit, &rule, action, denied_token);
    GateVerdict {
        outcome: gate_outcome(decision.outcome),
        layer: decision.layer,
        rule,
        read_only: verdict.read_only,
        params,
    }
}
