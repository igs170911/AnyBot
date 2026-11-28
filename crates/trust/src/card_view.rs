use anybot_protocol::{CardButton, CardKind, ComputerCaps, ReasonPayload, RuleLayer};

use crate::action::GateAction;
use crate::card_buttons::{AskOrigin, button_set};
use crate::card_data::build_card_for_computer;
use crate::card_reason::{always_payload, build_notes, build_reason};
use crate::gate_engine::GateLayer;
use crate::gate_flow::GateVerdict;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardView {
    pub action: String,
    pub actions: Vec<String>,
    pub kind: CardKind,
    pub buttons: Vec<CardButton>,
    pub reason: ReasonPayload,
    pub always_label: Option<ReasonPayload>,
    pub notes: Vec<ReasonPayload>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CardViewError {
    #[error("card has no concrete action")]
    MissingAction,
}

pub fn card_view(
    verdict: &GateVerdict,
    action: &GateAction,
    caps: &ComputerCaps,
    computer: &str,
) -> Result<CardView, CardViewError> {
    let data = build_card_for_computer(action, caps, computer);
    let Some(primary_action) = data.actions.first().cloned() else {
        return Err(CardViewError::MissingAction);
    };
    let origin = ask_origin(verdict);
    let buttons = button_set(&origin);
    let kind = card_kind(&origin);
    let reason = build_reason(&origin, verdict, action);
    let always_label = if buttons.contains(&CardButton::AlwaysAllow) {
        always_payload(action, caps, computer)
    } else {
        None
    };
    let notes = build_notes(action, caps, &buttons);
    Ok(CardView {
        action: primary_action,
        actions: data.actions,
        kind,
        buttons,
        reason,
        always_label,
        notes,
        classes: data.classes,
    })
}

fn ask_origin(verdict: &GateVerdict) -> AskOrigin {
    match verdict.layer {
        GateLayer::Floor => AskOrigin::AdminOrFloor {
            layer: RuleLayer::Floor,
        },
        GateLayer::Admin => AskOrigin::AdminOrFloor {
            layer: RuleLayer::Admin,
        },
        GateLayer::User => match verdict.gate_step() {
            anybot_protocol::GateStep::UserHooks => AskOrigin::UserHook,
            anybot_protocol::GateStep::AskPatterns => AskOrigin::UserAskRule {
                rule: ask_rule_name(verdict),
            },
            anybot_protocol::GateStep::Initiator => AskOrigin::Untrusted,
            anybot_protocol::GateStep::Computer
            | anybot_protocol::GateStep::Floor
            | anybot_protocol::GateStep::Admin
            | anybot_protocol::GateStep::ComputerUse
            | anybot_protocol::GateStep::AllowLists
            | anybot_protocol::GateStep::DefaultRule
            | anybot_protocol::GateStep::Relax
            | anybot_protocol::GateStep::AskYou => AskOrigin::Default,
        },
        GateLayer::Default => {
            if is_every_time_rule(&verdict.rule) {
                AskOrigin::EveryTime
            } else {
                AskOrigin::Default
            }
        }
    }
}

fn ask_rule_name(verdict: &GateVerdict) -> String {
    verdict
        .params
        .iter()
        .find(|(k, _)| k == "rule")
        .map_or_else(|| verdict.rule.clone(), |(_, v)| v.clone())
}

fn is_every_time_rule(rule: &str) -> bool {
    matches!(
        rule,
        "tool-unreadable-shape"
            | "user-machine-no-sandbox"
            | "user-machine-outside-write"
            | "user-machine-ask-every-time"
            | "user-machine-files"
            | "bot-machine-delete-unknown"
            | "bot-machine-outside-delete"
            | "computer-no-sandbox"
            | "floor-deny"
    )
}

fn card_kind(origin: &AskOrigin) -> CardKind {
    match origin {
        AskOrigin::EveryTime
        | AskOrigin::AdminOrFloor {
            layer: RuleLayer::Floor,
        } => CardKind::EveryTime,
        AskOrigin::Default
        | AskOrigin::UserAskRule { .. }
        | AskOrigin::UserHook
        | AskOrigin::AdminOrFloor { .. }
        | AskOrigin::Untrusted => CardKind::Action,
    }
}
