use std::sync::atomic::Ordering;

use crate::fake_helpers::{
    MISSING_FILE_REASON, NO_SCRIPT_REASON, apply_writes, target_kind, write_outcome,
};
use anybot_protocol::{
    ActualResult, CommandOutcome, ComputerCaps, ComputerFailure, ListOutcome, PathOutcome,
    ReadOutcome, SearchEngine, SearchRequest, TargetKind,
};
use anybot_trust::{GateConfig, GatedComputer, OpKind, OpRejection, OpTicket, normalize_command};

use crate::fake_files::digest_text;

use super::FakeComputer;

impl FakeComputer {
    pub(crate) fn bound_path_still(&self, ticket: &OpTicket) -> bool {
        let Some(path) = ticket.path() else {
            return true;
        };
        let current = Self::guard(&self.tree).canonical(path);
        let mut bound = Self::guard(&self.bound);
        if let Some(seen) = bound.get(path) {
            return *seen == current;
        }
        bound.insert(path.to_owned(), current);
        true
    }

    pub(crate) fn accept(&self, ticket: &OpTicket, kind: OpKind) -> Result<(), OpRejection> {
        if self.shift_declaration.swap(false, Ordering::SeqCst) {
            "/".clone_into(&mut Self::guard(&self.workdir));
        }
        let workdir = Self::guard(&self.workdir).clone();
        if ticket.fits(&self.name, kind)
            && ticket.matches_computer(&self.name, &self.caps, &workdir)
        {
            return Ok(());
        }
        Err(OpRejection::mismatch(format!(
            "the ticket is for {name} doing a {kind:?} operation",
            name = ticket.computer(),
            kind = ticket.kind(),
        )))
    }
}

impl GatedComputer for FakeComputer {
    fn gate_config(&self) -> GateConfig {
        Self::guard(&self.gate).clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn declared_caps(&self) -> ComputerCaps {
        self.caps.clone()
    }

    fn capabilities(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept(&ticket, OpKind::Caps)?;
        self.record("capabilities".to_owned());
        self.reachable()?;
        Ok(ActualResult::Caps(self.caps.clone()))
    }

    fn run_command(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept(&ticket, OpKind::Run)?;
        let Some(req) = ticket.command_request() else {
            return Err(OpRejection::mismatch(
                "the ticket has no command request".to_owned(),
            ));
        };
        let normalized = normalize_command(&req.command);
        self.record(format!("run_command {normalized}"));
        self.reachable()?;
        let Some(script) = self.scripts.lookup(&req.command) else {
            return Err(OpRejection::from_failure(ComputerFailure::operation(
                NO_SCRIPT_REASON.to_owned(),
            )));
        };
        let env = anybot_trust::apply_env_plan(&self.inherited_env, &req.env);
        Self::guard(&self.env_seen).insert(normalized, env.keys().cloned().collect());
        let output = if script.expand_env {
            expand_vars(&script.output, &env)
        } else {
            script.output.clone()
        };
        let (elapsed_ms, exit_code, wakes) = self.measured(script, req.timeout_ms);
        self.record(format!("run_command done wakes {wakes}"));
        apply_writes(&self.tree, &script.writes);
        Ok(ActualResult::Command(CommandOutcome {
            command: req.command,
            computer: self.name.clone(),
            exit_code: Some(exit_code),
            output,
            elapsed_ms,
        }))
    }

    fn read_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Read)?;
        let path = Self::ticket_path(&ticket)?;
        self.record(format!("read_file {path}"));
        self.reachable()?;
        let canonical = self.resolve_target(path).unwrap_or_else(|| path.to_owned());
        if Self::guard(&self.tree).is_directory(path) {
            return Ok(ActualResult::Read(ReadOutcome {
                path: canonical,
                kind: TargetKind::Directory,
                digest: digest_text(""),
                size: 0,
                content: String::new(),
            }));
        }
        let content = Self::guard(&self.tree).read(path).ok_or_else(|| {
            OpRejection::from_failure(ComputerFailure::operation(MISSING_FILE_REASON.to_owned()))
        })?;
        Ok(ActualResult::Read(ReadOutcome {
            path: canonical,
            kind: TargetKind::File,
            digest: digest_text(&content),
            size: content.len() as u64,
            content,
        }))
    }

    fn list_dir(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::ListDir)?;
        let path = Self::ticket_path(&ticket)?;
        self.record(format!("list_dir {path}"));
        self.reachable()?;
        let canonical = self.resolve_target(path).unwrap_or_else(|| path.to_owned());
        let entries = Self::guard(&self.tree)
            .list_entries(path)
            .map_err(|reason| OpRejection::from_failure(ComputerFailure::operation(reason)))?;
        Ok(ActualResult::List(ListOutcome {
            path: path.to_owned(),
            canonical,
            entries,
        }))
    }

    fn search(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Search)?;
        let path = Self::ticket_path(&ticket)?.to_owned();
        let (pattern, files_only) = ticket.search_query().unwrap_or(("", false));
        let pattern = pattern.to_owned();
        self.record(format!("search {path} {pattern}"));
        self.reachable()?;
        let engine = if self.ripgrep.load(Ordering::SeqCst) {
            SearchEngine::Ripgrep
        } else {
            SearchEngine::Plain
        };
        let step_ms = self.search_step_ms.load(Ordering::SeqCst);
        let query = SearchRequest {
            pattern,
            path,
            files_only,
        };
        let outcome = {
            let tree = Self::guard(&self.tree);
            crate::fake_search::run_search(&tree, &self.clock, engine, &query, step_ms)
        }
        .map_err(|reason| OpRejection::from_failure(ComputerFailure::operation(reason)))?;
        Ok(ActualResult::Search(outcome))
    }

    fn write_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Write)?;
        let path = Self::ticket_path(&ticket)?.to_owned();
        let content = ticket.content().unwrap_or("").to_owned();
        self.record(format!("write_file {}", ticket.path().unwrap_or(&path)));
        self.reachable()?;
        let before = Self::guard(&self.tree).digest(&path);
        let outcome = write_outcome(&self.tree, &path, &content, before);
        self.apply_mutation(&path, &content);
        Ok(ActualResult::Write(outcome))
    }

    fn edit_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Edit)?;
        let path = Self::ticket_path(&ticket)?.to_owned();
        let (old_text, new_text) = ticket.texts().unwrap_or(("", ""));
        self.record(format!("edit_file {path}"));
        self.reachable()?;
        let before = Self::guard(&self.tree).digest(&path);
        let content = self.apply_edit_exact(&path, old_text, new_text)?;
        let outcome = write_outcome(&self.tree, &path, &content, before);
        self.apply_mutation(&path, &content);
        Ok(ActualResult::Write(outcome))
    }

    fn path_info(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Path)?;
        let path = Self::ticket_path(&ticket)?;
        self.record(format!("path_info {path}"));
        self.reachable()?;
        let tree = Self::guard(&self.tree);
        let canonical = tree.canonical(path);
        let kind = target_kind(&tree, path);
        Ok(ActualResult::Path(PathOutcome {
            path: path.to_owned(),
            canonical,
            kind,
        }))
    }

    fn declared_workdir(&self) -> String {
        Self::guard(&self.workdir).clone()
    }

    fn resolve_target(&self, path: &str) -> Option<String> {
        if self.unresolvable.iter().any(|p| p == path) {
            return None;
        }
        Some(Self::guard(&self.tree).canonical(path))
    }
}

fn expand_vars(template: &str, env: &std::collections::BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while !rest.is_empty() {
        if let Some(after_dollar) = rest.strip_prefix('$') {
            let name_len = var_name_len(after_dollar);
            if name_len > 0
                && let (Some(name), Some(tail)) =
                    (after_dollar.get(..name_len), after_dollar.get(name_len..))
            {
                if let Some(val) = env.get(name) {
                    out.push_str(val);
                }
                rest = tail;
                continue;
            }
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        out.push(ch);
        rest = rest.get(ch.len_utf8()..).unwrap_or("");
    }
    out
}

fn var_name_len(text: &str) -> usize {
    let mut len = 0;
    for (idx, b) in text.bytes().enumerate() {
        let valid = if idx == 0 {
            b.is_ascii_alphabetic() || b == b'_'
        } else {
            b.is_ascii_alphanumeric() || b == b'_'
        };
        if !valid {
            break;
        }
        len = idx.saturating_add(1);
    }
    len
}
