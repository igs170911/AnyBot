use crate::action::{GateAction, PathView};
use crate::command_target::{DeleteBase, command_targets};
use crate::evidence::ActionContext;
use crate::safety_n5::{ActionClass, classes_of_command};

use crate::action::ActionSubject;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestedAction {
    Command {
        line: String,
        timeout_ms: u64,
    },
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    EditFile {
        path: String,
        old_text: String,
        new_text: String,
    },
    PathCheck {
        path: String,
    },
    Capabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRequest {
    pub context: ActionContext,
    pub subject: RequestedAction,
}

impl ActionRequest {
    #[must_use]
    pub fn gate_action(&self) -> GateAction {
        GateAction {
            tool: self.context.tool.clone(),
            subject: subject_of(&self.subject),
            extra_paths: Vec::new(),
            workdir: self.context.workdir.clone(),
        }
    }

    #[must_use]
    pub fn delete_targets(&self) -> Vec<PathView> {
        let RequestedAction::Command { line, .. } = &self.subject else {
            return Vec::new();
        };
        if !classes_of_command(line).contains(&ActionClass::Delete) {
            return Vec::new();
        }
        command_targets(line)
            .into_iter()
            .map(|target| view_of_target(&target))
            .collect()
    }
}

fn subject_of(requested: &RequestedAction) -> ActionSubject {
    match requested {
        RequestedAction::Command { line, timeout_ms } => ActionSubject::Command {
            line: line.clone(),
            timeout_ms: *timeout_ms,
        },
        RequestedAction::ReadFile { path } => ActionSubject::ReadFile {
            path: PathView::raw(path),
        },
        RequestedAction::WriteFile { path, content } => ActionSubject::WriteFile {
            path: PathView::raw(path),
            content: content.clone(),
        },
        RequestedAction::EditFile {
            path,
            old_text,
            new_text,
        } => ActionSubject::EditFile {
            path: PathView::raw(path),
            old_text: old_text.clone(),
            new_text: new_text.clone(),
        },
        RequestedAction::PathCheck { path } => ActionSubject::PathCheck {
            path: PathView::raw(path),
        },
        RequestedAction::Capabilities => ActionSubject::Capabilities,
    }
}

fn view_of_target(target: &crate::command_target::DeleteTarget) -> PathView {
    if crate::command_target::has_expansion_token(&target.token)
        || target.base == DeleteBase::Unresolved
    {
        return PathView::unresolvable(&target.token);
    }
    if target.base == DeleteBase::Cd && !target.token.starts_with('/') {
        return PathView::raw(&format!("{}/{}", target.cd_path, target.token));
    }
    PathView::raw(&target.token)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateAnswer {
    Allow,
    Deny,
}
