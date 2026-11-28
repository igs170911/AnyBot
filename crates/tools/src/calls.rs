use anybot_trust::{ActionContext, ActionRequest, RequestedAction};

use crate::arguments::{ReadArgs, ShellArgs, ToolArgs};
use crate::registry::{TOOL_EDIT_FILE, TOOL_LIST_DIR, TOOL_READ_FILE, TOOL_SHELL};

pub const DEFAULT_SHELL_TIMEOUT_MS: u64 = 30_000;
pub const MAX_SHELL_TIMEOUT_MS: u64 = 600_000;

#[must_use]
pub fn clamp_timeout(requested: Option<u64>) -> u64 {
    match requested {
        Some(value) => value.min(MAX_SHELL_TIMEOUT_MS),
        None => DEFAULT_SHELL_TIMEOUT_MS,
    }
}

#[must_use]
pub fn subject_of(tool: &str, args: ToolArgs) -> RequestedAction {
    match (tool, args) {
        (TOOL_EDIT_FILE, ToolArgs::Edit(edit)) => RequestedAction::EditFile {
            path: edit.path,
            old_text: edit.old_text,
            new_text: edit.new_text,
        },
        (TOOL_READ_FILE, ToolArgs::Read(read)) => RequestedAction::ReadFile { path: read.path },
        (TOOL_LIST_DIR, ToolArgs::List(list)) => RequestedAction::ListDir { path: list.path },
        (TOOL_SHELL, ToolArgs::Shell(shell)) => RequestedAction::Command {
            line: shell.line,
            timeout_ms: clamp_timeout(shell.timeout_ms),
        },
        _ => RequestedAction::Capabilities,
    }
}

#[must_use]
pub fn read_limits(args: &ReadArgs) -> (usize, usize) {
    (args.offset, args.limit)
}

#[must_use]
pub fn shell_line(args: &ShellArgs) -> &str {
    &args.line
}

#[must_use]
pub fn build_request(tool: &str, args: ToolArgs, context: ActionContext) -> ActionRequest {
    ActionRequest {
        context,
        subject: subject_of(tool, args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arguments::EditArgs;

    #[test]
    fn t_files_shell_7() {
        assert_eq!(clamp_timeout(Some(700_000)), 600_000);
        assert_eq!(clamp_timeout(Some(600_000)), 600_000);
        assert_eq!(clamp_timeout(Some(1)), 1);
        assert_eq!(clamp_timeout(None), 30_000);
    }

    #[test]
    fn test_file_edit_call_maps_to_the_gated_write_subject() {
        let subject = subject_of(
            TOOL_EDIT_FILE,
            ToolArgs::Edit(EditArgs {
                path: "notes.md".to_owned(),
                old_text: String::new(),
                new_text: "hello".to_owned(),
            }),
        );
        let RequestedAction::EditFile {
            path,
            old_text,
            new_text,
        } = subject
        else {
            panic!("edit subject missing: {subject:?}");
        };
        assert_eq!(path, "notes.md");
        assert_eq!(old_text, "");
        assert_eq!(new_text, "hello");
    }
}
