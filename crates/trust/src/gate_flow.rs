use crate::action::{ActionSubject, GateAction};
use crate::gate_engine::{GateLayer, Opinion};
use crate::gate_full::{EngineInput, ModeFlags, StyleFlags, decide};
use crate::machine_rules::{RuleVerdict, default_rule};
use crate::pass::Pass;
use crate::trust_floor::{FloorAction, FloorDeny, floor_check};
use anybot_foundation::TimestampMs;
use anybot_protocol::{
    ComputerCaps, ComputerOwner, GateNo, GateOutcome, GateRecord, JudgementSource, RuleLayer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateConfig {
    pub modes: ModeFlags,
    pub styles: StyleFlags,
    pub admin_hook: Opinion,
    pub user_hook: Opinion,
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
            styles: StyleFlags {
                ask_style_hit: false,
                allow_style_hit: false,
                always_hit: false,
                session_allowed: false,
            },
            admin_hook: Opinion::None,
            user_hook: Opinion::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateVerdict {
    pub outcome: GateOutcome,
    pub layer: GateLayer,
    pub rule: String,
    pub read_only: bool,
    pub params: Vec<(String, String)>,
}

impl GateVerdict {
    #[must_use]
    pub fn gate_no(&self) -> GateNo {
        GateNo(match self.layer {
            GateLayer::Floor => 1,
            GateLayer::Admin => 2,
            GateLayer::User => 3,
            GateLayer::Default => 4,
        })
    }

    #[must_use]
    pub fn record(&self) -> GateRecord {
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
    pub fn pass(&self, action: &GateAction, issued_at: TimestampMs) -> Option<Pass> {
        if self.outcome != GateOutcome::Allow {
            return None;
        }
        Some(Pass::issue(action.key(), self.record(), issued_at))
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
        ActionSubject::ReadFile { path } | ActionSubject::WriteFile { path, content: _ } => {
            FloorAction::FilePath {
                path: path.raw.clone(),
            }
        }
        ActionSubject::PathCheck { path } => FloorAction::FilePath {
            path: path.raw.clone(),
        },
        ActionSubject::Capabilities => FloorAction::FilePath {
            path: String::new(),
        },
    }
}

#[must_use]
pub fn judge(action: &GateAction, caps: &ComputerCaps, config: &GateConfig) -> GateVerdict {
    let verdict = default_rule(action, caps);
    let input = EngineInput {
        action: floor_view(action, caps),
        modes: config.modes.clone(),
        styles: config.styles.clone(),
        admin_hook: config.admin_hook,
        user_hook: config.user_hook,
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
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => None,
    }
}

fn path_label(action: &GateAction) -> String {
    match &action.subject {
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::PathCheck { path } => path.effective(&action.workdir),
        ActionSubject::Command { .. } | ActionSubject::Capabilities => String::new(),
    }
}

#[must_use]
pub fn rule_code(rule: &str) -> String {
    match rule {
        "sandbox-deny" => "gate-command-denied".to_owned(),
        "private-file" => "gate-private-file".to_owned(),
        "computer-no-sandbox" | "user-machine-no-sandbox" => "gate-no-sandbox".to_owned(),
        "bot-machine-outside" => "gate-bot-machine-outside".to_owned(),
        "user-machine-outside-write" => "gate-outside-write-denied".to_owned(),
        _ => "gate-denied-by-rule".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::PathView;
    use anybot_protocol::{CapState, ComputerPlatform};

    fn bot_caps() -> ComputerCaps {
        ComputerCaps {
            owner: ComputerOwner::Bot,
            platform: ComputerPlatform::Linux,
            run_commands: CapState::Supported,
            read_files: CapState::Supported,
            write_files: CapState::Supported,
            check_path: CapState::Supported,
            command_sandbox: CapState::Supported,
            ..ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Bot)
        }
    }

    #[test]
    fn test_gate_flow_floor_denial_records_the_first_gate() {
        let action = GateAction::command("shell", "mkfs.ext4 /dev/sda1", "/workspace", Vec::new());
        let verdict = judge(&action, &bot_caps(), &GateConfig::plain());
        assert_eq!(verdict.outcome, GateOutcome::Deny);
        assert_eq!(verdict.gate_no().value(), 1);
        assert_eq!(verdict.record().layer, RuleLayer::Floor);
    }

    #[test]
    fn test_gate_flow_default_direct_rule_issues_a_pass_for_that_action() {
        let action = GateAction::command("shell", "ls -la", "/workspace", Vec::new());
        let verdict = judge(&action, &bot_caps(), &GateConfig::plain());
        assert_eq!(verdict.outcome, GateOutcome::Allow);
        assert_eq!(verdict.rule, "bot-machine-command");
        let pass = verdict.pass(&action, TimestampMs::from_millis(5));
        assert!(pass.is_some());
        let other = GateAction::command("shell", "ls -lb", "/workspace", Vec::new());
        assert!(!pass.unwrap().authorizes(&other.key()));
    }

    #[test]
    fn test_gate_flow_private_path_is_denied_by_the_floor_rule() {
        let action = GateAction {
            tool: "read_file".to_owned(),
            subject: ActionSubject::ReadFile {
                path: PathView::raw("~/.ssh/id_ed25519"),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        };
        let verdict = judge(&action, &bot_caps(), &GateConfig::plain());
        assert_eq!(verdict.outcome, GateOutcome::Deny);
        assert!(floor_check(&floor_view(&action, &bot_caps())).is_some());
    }
}
