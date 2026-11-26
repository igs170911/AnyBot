use std::sync::Mutex;

use anybot_protocol::ActualResult;
use anybot_trust::{ActionKey, GatedComputer, OpRejection, Pass};

use crate::fake_computer::FakeComputer;

#[derive(Debug, Default)]
pub(crate) struct PassSpy {
    seen: Vec<bool>,
}

impl PassSpy {
    pub(crate) fn note(&mut self, authorized_other: bool) {
        self.seen.push(authorized_other);
    }
}

pub(crate) struct SpyComputer {
    inner: FakeComputer,
    spy: Mutex<PassSpy>,
}

impl SpyComputer {
    pub(crate) fn new(inner: FakeComputer) -> Self {
        SpyComputer {
            inner,
            spy: Mutex::new(PassSpy::default()),
        }
    }

    fn watch(&self, pass: &Pass) {
        let other = ActionKey::command("shell", "pwd");
        let mut spy = self
            .spy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        spy.note(pass.authorizes(&other));
    }

    pub(crate) fn seen(&self) -> Vec<bool> {
        self.spy
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .seen
            .clone()
    }
}

impl GatedComputer for SpyComputer {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn declared_caps(&self) -> anybot_protocol::ComputerCaps {
        self.inner.declared_caps()
    }

    fn capabilities(&self, pass: &Pass) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.capabilities(pass)
    }

    fn resolve_target(&self, path: &str) -> Option<String> {
        self.inner.resolve_target(path)
    }

    fn declared_workdir(&self) -> String {
        self.inner.declared_workdir()
    }

    fn path_info(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.path_info(pass, path)
    }

    fn read_file(&self, pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.read_file(pass, path)
    }

    fn write_file(
        &self,
        pass: &Pass,
        path: &str,
        content: &str,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.write_file(pass, path, content)
    }

    fn edit_file(
        &self,
        pass: &Pass,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.edit_file(pass, path, old_text, new_text)
    }

    fn run_command(
        &self,
        pass: &Pass,
        command: &str,
        timeout_ms: u64,
    ) -> Result<ActualResult, OpRejection> {
        self.watch(pass);
        self.inner.run_command(pass, command, timeout_ms)
    }
}
