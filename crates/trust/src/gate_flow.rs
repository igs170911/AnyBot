use crate::action::{ActionKey, ActionSubject, GateAction};
use crate::always_list::AlwaysList;
use crate::gate_engine::{GateLayer, Opinion};
use crate::gate_full::{EngineInput, ModeFlags, decide};
use crate::gate_rules::{hook_opinion, style_hits};
use crate::machine_rules::{RuleVerdict, default_rule};
use crate::op_ticket::OpTicket;
use crate::trust_floor::{FloorAction, FloorDeny, floor_check};
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    ComputerCaps, ComputerOwner, GateNo, GateOutcome, GateRecord, JudgementSource, RuleLayer,
};

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
}

impl GateConfig {
    #[must_use]
    pub fn plain() -> Self {
        GateConfig {
            modes: ModeFlags {
                six_class: false,
                ask_every_time: false,
                yolo: false,
                direct_mode: false,
                mdm_locks_direct: false,
            },
            user: UserRules::default(),
            admin: AdminRules::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateVerdict {
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
        | ActionSubject::PathCheck { path } => FloorAction::FilePath {
            path: path.raw.clone(),
        },
        ActionSubject::Capabilities => FloorAction::FilePath {
            path: String::new(),
        },
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
    let input = EngineInput {
        action: floor_view(action, caps),
        modes: config.modes.clone(),
        styles: style_hits(action, computer, &config.user),
        admin_hook: hook_opinion(&config.admin.hook, &action.key()),
        user_hook: hook_opinion(&config.user.hook, &action.key()),
        default_rule: verdict.opinion(),
    };
    let floor_hit = floor_check(&floor_view(action, caps));

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

fn verdict_params(
    layer: GateLayer,
    floor_hit: Option<FloorDeny>,
    rule: &str,
    action: &GateAction,
    denied_token: Option<&'static str>,
) -> Vec<(String, String)> {
    match (layer, floor_hit, denied_token) {
        (GateLayer::Floor, Some(FloorDeny::DeniedCommand), Some(token)) => {
            vec![("token".to_owned(), token.to_owned())]
        }
        (GateLayer::Floor, Some(FloorDeny::PrivateFile), _) => {
            vec![("path".to_owned(), path_label(action))]
        }
        (GateLayer::Floor, Some(FloorDeny::NoSandbox), _) => {
            vec![("capability".to_owned(), "commands".to_owned())]
        }
        _ if rule == "bot-machine-outside" || rule == "user-machine-outside-write" => {
            vec![("path".to_owned(), path_label(action))]
        }
        _ => Vec::new(),
    }
}

fn floor_denied_token(action: &GateAction, bot_machine: bool) -> Option<&'static str> {
    match &action.subject {
        ActionSubject::Command { line, .. } => crate::sandbox_deny::deny_token(line, bot_machine),
        ActionSubject::ReadFile { .. }
        | ActionSubject::WriteFile { .. }
        | ActionSubject::EditFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => None,
    }
}

fn path_label(action: &GateAction) -> String {
    match &action.subject {
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => path.effective(&action.workdir),
        ActionSubject::Command { .. } | ActionSubject::Capabilities => String::new(),
    }
}

#[must_use]
pub fn rule_code(rule: &str) -> String {
    match rule {
        "sandbox-deny" => crate::message_codes::CODE_GATE_COMMAND_DENIED.to_owned(),
        "private-file" => crate::message_codes::CODE_GATE_PRIVATE_FILE.to_owned(),
        "computer-no-sandbox" | "user-machine-no-sandbox" => {
            crate::message_codes::CODE_GATE_NO_SANDBOX.to_owned()
        }
        "bot-machine-outside" => crate::message_codes::CODE_GATE_BOT_MACHINE_OUTSIDE.to_owned(),
        "user-machine-outside-write" => {
            crate::message_codes::CODE_GATE_OUTSIDE_WRITE_DENIED.to_owned()
        }
        "tool-unreadable-eval"
        | "tool-unreadable-quotes"
        | "tool-unreadable-heredoc"
        | "tool-unreadable-head" => rule.to_owned(),
        _ => crate::message_codes::CODE_GATE_DENIED_BY_RULE.to_owned(),
    }
}

fn gate_outcome(outcome: crate::gate_engine::Outcome) -> GateOutcome {
    match outcome {
        crate::gate_engine::Outcome::Deny => GateOutcome::Deny,
        crate::gate_engine::Outcome::Ask => GateOutcome::Ask,
        crate::gate_engine::Outcome::Allow => GateOutcome::Allow,
    }
}

fn decision_rule(layer: GateLayer, verdict: &RuleVerdict, floor_hit: Option<FloorDeny>) -> String {
    match layer {
        GateLayer::Default => verdict.rule.to_owned(),
        GateLayer::Floor => match floor_hit {
            Some(deny) => deny.to_string(),
            None => "floor-deny".to_owned(),
        },
        GateLayer::Admin => "admin-rule".to_owned(),
        GateLayer::User => "user-rule".to_owned(),
    }
}
