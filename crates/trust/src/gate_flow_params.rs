use anybot_protocol::GateOutcome;

use crate::action::{ActionSubject, GateAction};
use crate::gate_engine::GateLayer;
use crate::machine_rules::RuleVerdict;
use crate::trust_floor::FloorDeny;

pub(crate) fn verdict_params(
    layer: GateLayer,
    floor_hit: Option<FloorDeny>,
    rule: &str,
    action: &GateAction,
    denied_token: Option<&'static str>,
    bot_name: &str,
    computer: &str,
) -> Vec<(String, String)> {
    match (layer, floor_hit, denied_token) {
        (GateLayer::Floor, Some(FloorDeny::DeniedCommand), Some(token)) => vec![
            ("token".to_owned(), token.to_owned()),
            ("command".to_owned(), command_label(action)),
        ],
        (GateLayer::Floor, Some(FloorDeny::PrivateFile), _) => {
            vec![("path".to_owned(), raw_path_label(action))]
        }
        (GateLayer::Floor, Some(FloorDeny::NoSandbox), _) => {
            vec![("capability".to_owned(), "commands".to_owned())]
        }
        _ if rule == "bot-machine-outside" => {
            let bot = if bot_name.is_empty() {
                computer
            } else {
                bot_name
            };
            vec![
                ("bot".to_owned(), bot.to_owned()),
                ("path".to_owned(), path_label(action)),
            ]
        }
        _ if rule == "user-machine-outside-write" => {
            vec![("path".to_owned(), path_label(action))]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn floor_denied_token(action: &GateAction, bot_machine: bool) -> Option<&'static str> {
    match &action.subject {
        ActionSubject::Command { line, .. } => crate::sandbox_deny::deny_token(line, bot_machine),
        ActionSubject::ReadFile { .. }
        | ActionSubject::WriteFile { .. }
        | ActionSubject::EditFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => None,
    }
}

fn command_label(action: &GateAction) -> String {
    match &action.subject {
        ActionSubject::Command { line, .. } => line.clone(),
        ActionSubject::ReadFile { .. }
        | ActionSubject::WriteFile { .. }
        | ActionSubject::EditFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => String::new(),
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

fn raw_path_label(action: &GateAction) -> String {
    match &action.subject {
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => path.raw_text().to_owned(),
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
        | "tool-unreadable-head"
        | "tool-unreadable-shape" => rule.to_owned(),
        _ => crate::message_codes::CODE_GATE_DENIED_BY_RULE.to_owned(),
    }
}

pub(crate) fn gate_outcome(outcome: crate::gate_engine::Outcome) -> GateOutcome {
    match outcome {
        crate::gate_engine::Outcome::Deny => GateOutcome::Deny,
        crate::gate_engine::Outcome::Ask => GateOutcome::Ask,
        crate::gate_engine::Outcome::Allow => GateOutcome::Allow,
    }
}

pub(crate) fn decision_rule(
    layer: GateLayer,
    verdict: &RuleVerdict,
    floor_hit: Option<FloorDeny>,
) -> String {
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
