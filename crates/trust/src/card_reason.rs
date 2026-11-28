use anybot_protocol::{CardButton, ComputerCaps, ComputerOwner, ReasonPayload, RuleLayer};

use crate::action::{ActionSubject, GateAction};
use crate::card_buttons::AskOrigin;
use crate::card_data::{has_hidden_shape, starts_interactive};
use crate::command_key::command_action_key;
use crate::gate_flow::GateVerdict;
use crate::machine_rules::{inside_workdir, is_outside_write};
use crate::message_codes::{
    CODE_ALWAYS_READ_OUTSIDE, CODE_ALWAYS_READ_WORKSPACE, CODE_ALWAYS_RUN_COMMAND,
    CODE_ALWAYS_RUN_COMMAND_BOT_COMPUTER, CODE_ALWAYS_WRITE_WORKSPACE, CODE_CARD_ADMIN_RULE,
    CODE_CARD_ASK_RULE, CODE_CARD_EDIT_FILE, CODE_CARD_EDIT_OUTSIDE, CODE_CARD_FLOOR_NO_SANDBOX,
    CODE_CARD_FLOOR_RULE, CODE_CARD_HIDDEN_SUBCOMMAND, CODE_CARD_INTERACTIVE_SHELL,
    CODE_CARD_NOTE_EVERY_TIME, CODE_CARD_NOTE_NO_SANDBOX_PRIVATE, CODE_CARD_READ_FILE,
    CODE_CARD_READ_OUTSIDE, CODE_CARD_RUN_COMMAND, CODE_CARD_UNPARSEABLE, CODE_CARD_USER_HOOK,
};
use crate::sandbox_levels::has_unbalanced_shape;

pub(crate) fn build_reason(
    origin: &AskOrigin,
    verdict: &GateVerdict,
    action: &GateAction,
) -> ReasonPayload {
    match origin {
        AskOrigin::UserAskRule { rule } => ReasonPayload {
            code: CODE_CARD_ASK_RULE.to_owned(),
            params: vec![("rule".to_owned(), rule.clone())],
        },
        AskOrigin::UserHook => ReasonPayload {
            code: CODE_CARD_USER_HOOK.to_owned(),
            params: vec![("rule".to_owned(), verdict.rule.clone())],
        },
        AskOrigin::AdminOrFloor {
            layer: RuleLayer::Admin,
        } => ReasonPayload {
            code: CODE_CARD_ADMIN_RULE.to_owned(),
            params: vec![("rule".to_owned(), verdict.rule.clone())],
        },
        AskOrigin::AdminOrFloor { .. } => floor_reason(verdict, action),
        AskOrigin::Default | AskOrigin::EveryTime | AskOrigin::Untrusted => action_reason(action),
    }
}

fn floor_reason(verdict: &GateVerdict, action: &GateAction) -> ReasonPayload {
    if verdict.rule == "computer-no-sandbox" {
        return ReasonPayload {
            code: CODE_CARD_FLOOR_NO_SANDBOX.to_owned(),
            params: vec![("command".to_owned(), command_line(action))],
        };
    }
    if verdict.rule == "tool-unreadable-shape" {
        return command_shape_reason(action);
    }
    ReasonPayload {
        code: CODE_CARD_FLOOR_RULE.to_owned(),
        params: vec![("rule".to_owned(), verdict.rule.clone())],
    }
}

fn command_line(action: &GateAction) -> String {
    match &action.subject {
        ActionSubject::Command { line, .. } => line.clone(),
        ActionSubject::ReadFile { .. }
        | ActionSubject::WriteFile { .. }
        | ActionSubject::EditFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => String::new(),
    }
}

fn command_shape_reason(action: &GateAction) -> ReasonPayload {
    let line = command_line(action);
    let code = if has_hidden_shape(&line) {
        CODE_CARD_HIDDEN_SUBCOMMAND
    } else if starts_interactive(&line) {
        CODE_CARD_INTERACTIVE_SHELL
    } else {
        CODE_CARD_UNPARSEABLE
    };
    ReasonPayload {
        code: code.to_owned(),
        params: Vec::new(),
    }
}

fn action_reason(action: &GateAction) -> ReasonPayload {
    match &action.subject {
        ActionSubject::Command { line, .. } => command_action_reason(line),
        ActionSubject::WriteFile { path, .. } | ActionSubject::EditFile { path, .. } => {
            let code = if is_outside_write(path, &action.workdir) {
                CODE_CARD_EDIT_OUTSIDE
            } else {
                CODE_CARD_EDIT_FILE
            };
            ReasonPayload {
                code: code.to_owned(),
                params: vec![("path".to_owned(), path.effective(&action.workdir))],
            }
        }
        ActionSubject::ReadFile { path } | ActionSubject::PathCheck { path } => {
            let code = if inside_workdir(path, &action.workdir) {
                CODE_CARD_READ_FILE
            } else {
                CODE_CARD_READ_OUTSIDE
            };
            ReasonPayload {
                code: code.to_owned(),
                params: vec![("path".to_owned(), path.effective(&action.workdir))],
            }
        }
        ActionSubject::Capabilities => ReasonPayload {
            code: CODE_CARD_UNPARSEABLE.to_owned(),
            params: Vec::new(),
        },
    }
}

fn command_action_reason(line: &str) -> ReasonPayload {
    if has_hidden_shape(line) {
        return ReasonPayload {
            code: CODE_CARD_HIDDEN_SUBCOMMAND.to_owned(),
            params: Vec::new(),
        };
    }
    if starts_interactive(line) {
        return ReasonPayload {
            code: CODE_CARD_INTERACTIVE_SHELL.to_owned(),
            params: Vec::new(),
        };
    }
    if has_unbalanced_shape(line) {
        return ReasonPayload {
            code: CODE_CARD_UNPARSEABLE.to_owned(),
            params: Vec::new(),
        };
    }
    ReasonPayload {
        code: CODE_CARD_RUN_COMMAND.to_owned(),
        params: vec![("command".to_owned(), line.to_owned())],
    }
}

pub(crate) fn always_payload(
    action: &GateAction,
    caps: &ComputerCaps,
    computer: &str,
) -> Option<ReasonPayload> {
    match &action.subject {
        ActionSubject::Command { line, .. } => {
            let words = command_action_key(line);
            if caps.owner == ComputerOwner::Bot {
                Some(ReasonPayload {
                    code: CODE_ALWAYS_RUN_COMMAND_BOT_COMPUTER.to_owned(),
                    params: vec![("words".to_owned(), words)],
                })
            } else {
                Some(ReasonPayload {
                    code: CODE_ALWAYS_RUN_COMMAND.to_owned(),
                    params: vec![
                        ("computer".to_owned(), computer.to_owned()),
                        ("words".to_owned(), words),
                    ],
                })
            }
        }
        ActionSubject::WriteFile { path, .. } | ActionSubject::EditFile { path, .. } => {
            if is_outside_write(path, &action.workdir) {
                None
            } else {
                Some(ReasonPayload {
                    code: CODE_ALWAYS_WRITE_WORKSPACE.to_owned(),
                    params: vec![("computer".to_owned(), computer.to_owned())],
                })
            }
        }
        ActionSubject::ReadFile { path } | ActionSubject::PathCheck { path } => {
            let code = if inside_workdir(path, &action.workdir) {
                CODE_ALWAYS_READ_WORKSPACE
            } else {
                CODE_ALWAYS_READ_OUTSIDE
            };
            Some(ReasonPayload {
                code: code.to_owned(),
                params: vec![("computer".to_owned(), computer.to_owned())],
            })
        }
        ActionSubject::Capabilities => None,
    }
}

pub(crate) fn build_notes(
    action: &GateAction,
    caps: &ComputerCaps,
    buttons: &[CardButton],
) -> Vec<ReasonPayload> {
    let mut notes = Vec::new();
    if !buttons.contains(&CardButton::AlwaysAllow) {
        notes.push(ReasonPayload {
            code: CODE_CARD_NOTE_EVERY_TIME.to_owned(),
            params: Vec::new(),
        });
    }
    if !caps.command_sandbox.yes() && matches!(action.subject, ActionSubject::Command { .. }) {
        notes.push(ReasonPayload {
            code: CODE_CARD_NOTE_NO_SANDBOX_PRIVATE.to_owned(),
            params: Vec::new(),
        });
    }
    notes
}
