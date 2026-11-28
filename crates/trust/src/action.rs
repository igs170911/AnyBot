use anybot_protocol::EnvPlan;

use crate::sandbox_deny::normalize_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathView {
    raw: String,
    resolved: Option<String>,
    resolvable: bool,
}

impl PathView {
    #[must_use]
    pub fn raw_text(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn is_resolvable(&self) -> bool {
        self.resolvable
    }

    #[must_use]
    pub fn resolved_text(&self) -> Option<&str> {
        self.resolved.as_deref()
    }

    pub(crate) fn raw(path: &str) -> Self {
        PathView {
            raw: path.to_owned(),
            resolved: None,
            resolvable: true,
        }
    }

    pub(crate) fn unresolvable(path: &str) -> Self {
        PathView {
            raw: path.to_owned(),
            resolved: None,
            resolvable: false,
        }
    }

    pub(crate) fn resolved(raw: &str, canonical: &str) -> Self {
        PathView {
            raw: raw.to_owned(),
            resolved: Some(canonical.to_owned()),
            resolvable: true,
        }
    }

    #[must_use]
    pub fn effective(&self, workdir: &str) -> String {
        if let Some(canonical) = &self.resolved {
            return canonical.clone();
        }
        if self.raw.starts_with('/') {
            return normalize(&self.raw);
        }
        let base = workdir.trim_end_matches('/');
        normalize(&format!("{base}/{}", self.raw))
    }
}

fn normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if let Some(last) = parts.last()
                && *last != ".."
            {
                parts.pop();
                continue;
            }
            if absolute {
                continue;
            }
            parts.push("..");
            continue;
        }
        parts.push(segment);
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        if joined.is_empty() {
            return ".".to_owned();
        }
        joined
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionSubject {
    Command {
        line: String,
        timeout_ms: u64,
        env: EnvPlan,
    },
    ReadFile {
        path: PathView,
    },
    ListDir {
        path: PathView,
    },
    Search {
        path: PathView,
        pattern: String,
        files_only: bool,
    },
    WriteFile {
        path: PathView,
        content: String,
    },
    EditFile {
        path: PathView,
        old_text: String,
        new_text: String,
    },
    PathCheck {
        path: PathView,
    },
    Capabilities,
}

pub const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 10_000;

impl ActionSubject {
    #[must_use]
    pub fn path_views(&self) -> Vec<&PathView> {
        match self {
            ActionSubject::ReadFile { path }
            | ActionSubject::ListDir { path }
            | ActionSubject::Search { path, .. }
            | ActionSubject::WriteFile { path, content: _ }
            | ActionSubject::EditFile { path, .. }
            | ActionSubject::PathCheck { path } => vec![path],
            ActionSubject::Command { .. } | ActionSubject::Capabilities => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateAction {
    pub tool: String,
    pub subject: ActionSubject,
    pub extra_paths: Vec<PathView>,
    pub workdir: String,
}

impl GateAction {
    #[must_use]
    pub fn command(tool: &str, line: &str, workdir: &str, extra_paths: Vec<PathView>) -> Self {
        GateAction {
            tool: tool.to_owned(),
            subject: ActionSubject::Command {
                line: line.to_owned(),
                timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
                env: crate::sandbox_env::command_env_plan(),
            },
            extra_paths,
            workdir: workdir.to_owned(),
        }
    }

    #[must_use]
    pub fn edit_file(
        tool: &str,
        path: &str,
        old_text: &str,
        new_text: &str,
        workdir: &str,
    ) -> Self {
        GateAction {
            tool: tool.to_owned(),
            subject: ActionSubject::EditFile {
                path: PathView::raw(path),
                old_text: old_text.to_owned(),
                new_text: new_text.to_owned(),
            },
            extra_paths: Vec::new(),
            workdir: workdir.to_owned(),
        }
    }

    #[must_use]
    pub fn edit(tool: &str, path: &str, content: &str, workdir: &str) -> Self {
        GateAction {
            tool: tool.to_owned(),
            subject: ActionSubject::WriteFile {
                path: PathView::raw(path),
                content: content.to_owned(),
            },
            extra_paths: Vec::new(),
            workdir: workdir.to_owned(),
        }
    }

    #[must_use]
    pub fn path_views(&self) -> Vec<&PathView> {
        let mut views = self.subject.path_views();
        views.extend(self.extra_paths.iter());
        views
    }

    #[must_use]
    pub fn key(&self) -> ActionKey {
        let subject = match &self.subject {
            ActionSubject::Command { line, .. } => normalize_command(line),
            ActionSubject::ReadFile { path } => format!("read:{}", path.raw_text()),
            ActionSubject::ListDir { path } => format!("list:{}", path.raw_text()),
            ActionSubject::Search {
                path,
                pattern,
                files_only,
            } => format!("search:{}:{files_only}:{pattern}", path.raw_text()),
            ActionSubject::WriteFile { path, content: _ } => format!("write:{}", path.raw_text()),
            ActionSubject::EditFile { path, .. } => format!("edit:{}", path.raw_text()),
            ActionSubject::PathCheck { path } => format!("path:{}", path.raw_text()),
            ActionSubject::Capabilities => "capabilities".to_owned(),
        };
        ActionKey {
            tool: self.tool.clone(),
            subject,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionKey {
    tool: String,
    subject: String,
}

impl ActionKey {
    #[must_use]
    pub fn command(tool: &str, line: &str) -> Self {
        ActionKey {
            tool: tool.to_owned(),
            subject: normalize_command(line),
        }
    }

    #[must_use]
    pub fn file(tool: &str, mode: &str, path: &str) -> Self {
        ActionKey {
            tool: tool.to_owned(),
            subject: format!("{mode}:{path}"),
        }
    }

    #[must_use]
    pub fn capabilities(tool: &str) -> Self {
        ActionKey {
            tool: tool.to_owned(),
            subject: "capabilities".to_owned(),
        }
    }

    #[must_use]
    pub fn parts(&self) -> (&str, &str) {
        (&self.tool, &self.subject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_action_key_normalizes_command_and_marks_file_modes() {
        let command = GateAction::command("shell", "LS   -la", "/workspace", Vec::new());
        assert_eq!(command.key().parts(), ("shell", "ls -la"));
        let read = GateAction {
            tool: "read_file".to_owned(),
            subject: ActionSubject::ReadFile {
                path: PathView::raw("notes.md"),
            },
            extra_paths: Vec::new(),
            workdir: "/workspace".to_owned(),
        };
        assert_eq!(read.key().parts(), ("read_file", "read:notes.md"));
    }

    #[test]
    fn test_path_view_resolved_target_wins_over_relative_raw_path() {
        let linked = PathView::resolved("link/data.txt", "/elsewhere/data.txt");
        assert_eq!(linked.effective("/workspace"), "/elsewhere/data.txt");
        let plain = PathView::raw("data.txt");
        assert_eq!(plain.effective("/workspace"), "/workspace/data.txt");
    }
}
