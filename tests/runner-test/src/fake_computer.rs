use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use anybot_foundation::ContentDigest;
use anybot_protocol::{
    ActualResult, BeforeDigest, CommandOutcome, ComputerCaps, ComputerFailure, PathOutcome,
    ReadOutcome, TargetKind, WriteOutcome,
};
use anybot_trust::{ActionKey, GatedComputer, OpRejection, Pass, normalize_command};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_files::{FileTree, digest_text};
use crate::fake_scripts::{CommandScript, ScriptTable};

pub const TIMEOUT_EXIT_CODE: i32 = 124;
pub const NO_SCRIPT_REASON: &str = "the fake computer has no script for this command";
pub const MISSING_FILE_REASON: &str = "no such file on the fake computer";
pub const UNREACHABLE_REASON: &str = "the fake computer is set unreachable";

pub struct FakeComputer {
    pub(crate) name: String,
    pub(crate) caps: ComputerCaps,
    pub(crate) clock: Arc<FakeClock>,
    pub(crate) tree: Mutex<FileTree>,
    pub(crate) scripts: ScriptTable,
    pub(crate) unreachable: AtomicBool,
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

    fn guard<T>(cell: &Mutex<T>) -> MutexGuard<'_, T> {
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

    fn authorize(pass: &Pass, key: &ActionKey) -> Result<(), OpRejection> {
        if !pass.authorizes(key) {
            let (tool, subject) = key.parts();
            return Err(OpRejection::unauthorized(format!(
                "pass does not authorize {tool} {subject}"
            )));
        }
        Ok(())
    }

    fn command_key(pass: &Pass, command: &str) -> ActionKey {
        ActionKey::command(pass.tool(), command)
    }

    fn file_key(pass: &Pass, mode: &str, path: &str) -> ActionKey {
        ActionKey::file(pass.tool(), mode, path)
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

    fn capabilities(&self, pass: &Pass) -> Result<ActualResult, OpRejection> {
        self.record(format!("capabilities {}", pass.tool()));
        Self::authorize(pass, &ActionKey::capabilities(pass.tool()))?;
        self.reachable()?;
        Ok(ActualResult::Caps(self.caps.clone()))
    }

    fn run_command(
        &self,
        pass: &Pass,
        command: &str,
        timeout_ms: u64,
    ) -> Result<ActualResult, OpRejection> {
        self.record(format!("run_command {}", normalize_command(command)));
        Self::authorize(pass, &Self::command_key(pass, command))?;
        self.reachable()?;
        let Some(script) = self.scripts.lookup(command) else {
            return Err(OpRejection::from_failure(ComputerFailure::operation(
                NO_SCRIPT_REASON.to_owned(),
            )));
        };
        let (elapsed_ms, exit_code, wakes) = self.measured(script, timeout_ms);
        self.record(format!("run_command done wakes {wakes}"));
        apply_writes(&self.tree, &script.writes);
        Ok(ActualResult::Command(CommandOutcome {
            command: command.to_owned(),
            computer: self.name.clone(),
            exit_code: Some(exit_code),
            output: script.output.clone(),
            elapsed_ms,
        }))
    }

    fn read_file(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.record(format!("read_file {path}"));
        Self::authorize(pass, &Self::file_key(pass, "read", path))?;
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

    fn write_file(
        &self,
        pass: &Pass,
        path: &str,
        content: &str,
    ) -> Result<ActualResult, OpRejection> {
        self.record(format!("write_file {path}"));
        Self::authorize(pass, &Self::file_key(pass, "write", path))?;
        self.reachable()?;
        let before = Self::guard(&self.tree).digest(path);
        let outcome = write_outcome(&self.tree, path, content, before);
        Ok(ActualResult::Write(outcome))
    }

    fn path_info(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.record(format!("path_info {path}"));
        Self::authorize(pass, &Self::file_key(pass, "path", path))?;
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

    fn resolve_target(&self, path: &str) -> Option<String> {
        Some(Self::guard(&self.tree).canonical(path))
    }
}

fn write_outcome(
    tree: &Mutex<FileTree>,
    path: &str,
    content: &str,
    before: Option<ContentDigest>,
) -> WriteOutcome {
    let canonical = {
        let guard = FakeComputer::guard(tree);
        guard.canonical(path)
    };
    let outcome = WriteOutcome {
        path: canonical,
        before: match before {
            Some(digest) => BeforeDigest::Present(digest),
            None => BeforeDigest::Absent,
        },
        after: digest_text(content),
        size: content.len() as u64,
    };
    FakeComputer::guard(tree).write(path, content);
    outcome
}

fn apply_writes(tree: &Mutex<FileTree>, writes: &[(String, String)]) {
    let mut guard = FakeComputer::guard(tree);
    for (path, content) in writes {
        guard.write(path, content);
    }
}

fn target_kind(tree: &FileTree, path: &str) -> Option<TargetKind> {
    if tree.is_directory(path) {
        return Some(TargetKind::Directory);
    }
    if tree.exists(path) {
        return Some(TargetKind::File);
    }
    None
}
