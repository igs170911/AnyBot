use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::fake_helpers::{
    MISSING_FILE_REASON, TIMEOUT_EXIT_CODE, UNREACHABLE_REASON, edit_rejection,
};
use anybot_foundation::ContentDigest;
use anybot_protocol::{ComputerCaps, ComputerFailure};
use anybot_trust::{OpKind, OpRejection, OpTicket};
use test_harness::FakeClock;

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_files::FileTree;
use crate::fake_scripts::{CommandScript, ScriptTable};

pub struct FakeComputer {
    pub(crate) gate: Mutex<anybot_trust::GateConfig>,
    pub(crate) bound: Mutex<std::collections::BTreeMap<String, String>>,
    pub(crate) name: String,
    pub(crate) caps: ComputerCaps,
    pub(crate) clock: Arc<FakeClock>,
    pub(crate) tree: Mutex<FileTree>,
    pub(crate) scripts: ScriptTable,
    pub(crate) unreachable: AtomicBool,
    pub(crate) unresolvable: Vec<String>,
    pub(crate) workdir: Mutex<String>,
    pub(crate) shift_declaration: AtomicBool,
    pub(crate) mutate_after_write: AtomicBool,
    pub(crate) log: Mutex<Vec<String>>,
}

impl FakeComputer {
    pub(crate) fn gate_config(&self) -> MutexGuard<'_, anybot_trust::GateConfig> {
        Self::guard(&self.gate)
    }

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

    pub fn set_mutate_after_write(&self, on: bool) {
        self.mutate_after_write.store(on, Ordering::SeqCst);
    }

    pub(crate) fn apply_mutation(&self, path: &str, content: &str) {
        if !self.mutate_after_write.load(Ordering::SeqCst) {
            return;
        }
        let changed = format!(
            "{content}
changed"
        );
        Self::guard(&self.tree).write_exact(path, &changed);
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

    pub(crate) fn record(&self, entry: String) {
        Self::guard(&self.log).push(entry);
    }

    pub(crate) fn reachable(&self) -> Result<(), OpRejection> {
        if self.is_unreachable() {
            return Err(OpRejection::from_failure(ComputerFailure::unreachable(
                UNREACHABLE_REASON.to_owned(),
            )));
        }
        Ok(())
    }

    pub(crate) fn accept_path(
        &self,
        ticket: &OpTicket,
        _name: &str,
        kind: OpKind,
    ) -> Result<(), OpRejection> {
        self.accept(ticket, kind)?;
        if Self::bound_path_still(self, ticket) {
            return Ok(());
        }
        Err(OpRejection::mismatch(
            "the path no longer resolves to the path on the ticket".to_owned(),
        ))
    }

    pub(crate) fn ticket_path(ticket: &OpTicket) -> Result<&str, OpRejection> {
        ticket
            .path()
            .ok_or_else(|| OpRejection::mismatch("the ticket has no path".to_owned()))
    }

    pub(crate) fn measured(&self, script: &CommandScript, timeout_ms: u64) -> (u64, i32, usize) {
        let started = self.clock.now_ms();
        let billed = script.duration_ms.min(timeout_ms);
        let wakes = self.clock.advance(Duration::from_millis(billed));
        let finished = self.clock.now_ms();
        let elapsed = finished.saturating_sub(started);
        let code = timed_out_code(script, billed, timeout_ms);
        (elapsed, code, wakes)
    }
    pub(crate) fn apply_edit_exact(
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
