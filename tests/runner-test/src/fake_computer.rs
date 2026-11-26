use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::fake_helpers::{
    MISSING_FILE_REASON, NO_SCRIPT_REASON, TIMEOUT_EXIT_CODE, UNREACHABLE_REASON, apply_writes,
    edit_rejection, target_kind, write_outcome,
};
use anybot_foundation::ContentDigest;
use anybot_protocol::{
    ActualResult, CommandOutcome, ComputerCaps, ComputerFailure, PathOutcome, ReadOutcome,
    TargetKind,
};
use anybot_trust::{GatedComputer, OpKind, OpRejection, OpTicket, normalize_command};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_files::{FileTree, digest_text};
use crate::fake_scripts::{CommandScript, ScriptTable};

pub struct FakeComputer {
    pub(crate) name: String,
    pub(crate) caps: ComputerCaps,
    pub(crate) clock: Arc<FakeClock>,
    pub(crate) tree: Mutex<FileTree>,
    pub(crate) scripts: ScriptTable,
    pub(crate) unreachable: AtomicBool,
    pub(crate) unresolvable: Vec<String>,
    pub(crate) workdir: Mutex<String>,
    pub(crate) shift_declaration: AtomicBool,
    pub(crate) log: Mutex<Vec<String>>,
}

impl FakeComputer {
    #[must_use]
    pub fn builder() -> FakeComputerBuilder {
        FakeComputerBuilder::default()
    }

    #[must_use]
    pub fn clock(&self) -> &Arc<FakeClock> {
        &self.clock
    }

    #[must_use]
    pub fn logged(&self) -> Vec<String> {
        Self::guard(&self.log).clone()
    }

    #[must_use]
    pub fn is_unreachable(&self) -> bool {
        self.unreachable.load(Ordering::SeqCst)
    }

    pub fn set_unreachable(&self, on: bool) {
        self.unreachable.store(on, Ordering::SeqCst);
    }

    pub fn retarget_link(&self, path: &str, target: &str) {
        Self::guard(&self.tree).link(path, target);
    }

    pub fn shift_declaration_before_the_operation(&self) {
        self.shift_declaration.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn file_digest(&self, path: &str) -> Option<ContentDigest> {
        Self::guard(&self.tree).digest(path)
    }

    #[must_use]
    pub fn file_content(&self, path: &str) -> Option<String> {
        Self::guard(&self.tree).read(path)
    }

    #[must_use]
    pub fn script(&self, command: &str) -> Option<&CommandScript> {
        self.scripts.lookup(command)
    }

    pub fn set_script(&mut self, command: &str, script: CommandScript) {
        self.scripts.insert(command, script);
    }

    pub(crate) fn guard<T>(cell: &Mutex<T>) -> MutexGuard<'_, T> {
        cell.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn record(&self, entry: String) {
        Self::guard(&self.log).push(entry);
    }

    fn reachable(&self) -> Result<(), OpRejection> {
        if self.is_unreachable() {
            return Err(OpRejection::from_failure(ComputerFailure::unreachable(
                UNREACHABLE_REASON.to_owned(),
            )));
        }
        Ok(())
    }

    fn accept(&self, ticket: &OpTicket, kind: OpKind) -> Result<(), OpRejection> {
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

    fn accept_path(&self, ticket: &OpTicket, _name: &str, kind: OpKind) -> Result<(), OpRejection> {
        self.accept(ticket, kind)?;
        if self.bound_path_still(ticket) {
            return Ok(());
        }
        Err(OpRejection::mismatch(
            "the path no longer resolves to the path on the ticket".to_owned(),
        ))
    }

    fn ticket_path(ticket: &OpTicket) -> Result<&str, OpRejection> {
        ticket
            .path()
            .ok_or_else(|| OpRejection::mismatch("the ticket has no path".to_owned()))
    }

    fn measured(&self, script: &CommandScript, timeout_ms: u64) -> (u64, i32, usize) {
        let started = self.clock.now_ms();
        let billed = script.duration_ms.min(timeout_ms);
        let wakes = self.clock.advance(Duration::from_millis(billed));
        let finished = self.clock.now_ms();
        let elapsed = finished.saturating_sub(started);
        let code = timed_out_code(script, billed, timeout_ms);
        (elapsed, code, wakes)
    }
    fn apply_edit_exact(
        &self,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<String, OpRejection> {
        let current = if old_text.is_empty() {
            Self::guard(&self.tree).read(path)
        } else {
            Some(Self::guard(&self.tree).read(path).ok_or_else(|| {
                OpRejection::from_failure(ComputerFailure::operation(
                    MISSING_FILE_REASON.to_owned(),
                ))
            })?)
        };
        let applied = anybot_trust::apply_edit(current.as_deref(), old_text, new_text)
            .map_err(|e| edit_rejection(&e))?;
        Ok(applied.content)
    }
}

fn timed_out_code(script: &CommandScript, billed_ms: u64, timeout_ms: u64) -> i32 {
    if script.duration_ms > timeout_ms && billed_ms == timeout_ms {
        TIMEOUT_EXIT_CODE
    } else {
        script.exit_code
    }
}

impl GatedComputer for FakeComputer {
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
        let (command, timeout_ms) = ticket.command_line().unwrap_or(("", 0));
        let command = command.to_owned();
        self.record(format!("run_command {}", normalize_command(&command)));
        self.reachable()?;
        let Some(script) = self.scripts.lookup(&command) else {
            return Err(OpRejection::from_failure(ComputerFailure::operation(
                NO_SCRIPT_REASON.to_owned(),
            )));
        };
        let (elapsed_ms, exit_code, wakes) = self.measured(script, timeout_ms);
        self.record(format!("run_command done wakes {wakes}"));
        apply_writes(&self.tree, &script.writes);
        Ok(ActualResult::Command(CommandOutcome {
            command,
            computer: self.name.clone(),
            exit_code: Some(exit_code),
            output: script.output.clone(),
            elapsed_ms,
        }))
    }

    fn read_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Read)?;
        let path = Self::ticket_path(&ticket)?;
        self.record(format!("read_file {path}"));
        self.reachable()?;
        let content = Self::guard(&self.tree).read(path).ok_or_else(|| {
            OpRejection::from_failure(ComputerFailure::operation(MISSING_FILE_REASON.to_owned()))
        })?;
        let canonical = self.resolve_target(path).unwrap_or_else(|| path.to_owned());
        Ok(ActualResult::Read(ReadOutcome {
            path: canonical,
            kind: TargetKind::File,
            digest: digest_text(&content),
            size: content.len() as u64,
            content,
        }))
    }

    fn write_file(&self, ticket: OpTicket) -> Result<ActualResult, OpRejection> {
        self.accept_path(&ticket, &self.name, OpKind::Write)?;
        let path = Self::ticket_path(&ticket)?.to_owned();
        let content = ticket.content().unwrap_or("").to_owned();
        self.record(format!("write_file {}", ticket.path().unwrap_or(&path)));
        self.reachable()?;
        let before = Self::guard(&self.tree).digest(&path);
        let outcome = write_outcome(&self.tree, &path, &content, before);
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
