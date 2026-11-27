use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use anybot_protocol::{ComputerCaps, ComputerOwner, ComputerPlatform};
use anybot_trust::GateConfig;
use test_harness::FakeClock;

use crate::fake_computer::FakeComputer;
use crate::fake_files::FileTree;
use crate::fake_scripts::{CommandScript, ScriptTable};

pub const DEFAULT_NAME: &str = "fake-computer";

pub struct FakeComputerBuilder {
    name: String,
    caps: ComputerCaps,
    clock: Arc<FakeClock>,
    tree: FileTree,
    scripts: ScriptTable,
    unreachable: bool,
    unresolvable: Vec<String>,
    workdir: String,
    gate: GateConfig,
}

impl Default for FakeComputerBuilder {
    fn default() -> Self {
        FakeComputerBuilder {
            name: DEFAULT_NAME.to_owned(),
            caps: ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::Unknown),
            clock: FakeClock::new(0),
            tree: FileTree::new(),
            scripts: ScriptTable::new(),
            unreachable: false,
            unresolvable: Vec::new(),
            workdir: "/workspace".to_owned(),
            gate: GateConfig::plain(),
        }
    }
}

impl FakeComputerBuilder {
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    #[must_use]
    pub fn caps(mut self, caps: ComputerCaps) -> Self {
        self.caps = caps;
        self
    }

    #[must_use]
    pub fn bot_machine(self) -> Self {
        let declared = ComputerCaps::declared(ComputerPlatform::BotMachine, ComputerOwner::Bot);
        self.caps(declared).commands().files()
    }

    #[must_use]
    pub fn commands(mut self) -> Self {
        self.caps.run_commands = anybot_protocol::CapState::Supported;
        self.caps.command_sandbox = anybot_protocol::CapState::Supported;
        self
    }

    #[must_use]
    pub fn files(mut self) -> Self {
        self.caps.read_files = anybot_protocol::CapState::Supported;
        self.caps.write_files = anybot_protocol::CapState::Supported;
        self.caps.check_path = anybot_protocol::CapState::Supported;
        self
    }

    #[must_use]
    pub fn sandbox(self, on: bool) -> Self {
        let caps = ComputerCaps {
            command_sandbox: if on {
                anybot_protocol::CapState::Supported
            } else {
                anybot_protocol::CapState::Unsupported
            },
            ..self.caps
        };
        self.caps(caps)
    }

    #[must_use]
    pub fn platform(self, platform: ComputerPlatform) -> Self {
        let caps = ComputerCaps::declared(platform, self.caps.owner);
        self.caps(caps)
    }

    #[must_use]
    pub fn user_machine(self) -> Self {
        let caps = ComputerCaps {
            owner: ComputerOwner::User,
            ..self.caps
        };
        self.caps(caps)
    }

    #[must_use]
    pub fn without_sandbox(self) -> Self {
        self.sandbox(false)
    }

    #[must_use]
    pub fn clock(mut self, clock: Arc<FakeClock>) -> Self {
        self.clock = clock;
        self
    }

    #[must_use]
    pub fn file(mut self, path: &str, content: &str) -> Self {
        self.tree.write(path, content);
        self
    }

    #[must_use]
    pub fn directory(mut self, path: &str) -> Self {
        self.tree.add_dir(path);
        self
    }

    #[must_use]
    pub fn link(mut self, path: &str, target: &str) -> Self {
        self.tree.link(path, target);
        self
    }

    #[must_use]
    pub fn script(mut self, command: &str, script: CommandScript) -> Self {
        self.scripts.insert(command, script);
        self
    }

    #[must_use]
    pub fn unresolvable(mut self, path: &str) -> Self {
        self.unresolvable.push(path.to_owned());
        self
    }

    #[must_use]
    pub fn workdir(mut self, workdir: &str) -> Self {
        workdir.clone_into(&mut self.workdir);
        self
    }

    #[must_use]
    pub fn gate(mut self, gate: GateConfig) -> Self {
        self.gate = gate;
        self
    }

    #[must_use]
    pub fn unreachable(mut self, on: bool) -> Self {
        self.unreachable = on;
        self
    }

    #[must_use]
    pub fn build(self) -> FakeComputer {
        FakeComputer {
            name: self.name,
            caps: self.caps,
            clock: self.clock,
            tree: Mutex::new(self.tree),
            scripts: self.scripts,
            unreachable: AtomicBool::new(self.unreachable),
            unresolvable: self.unresolvable,
            workdir: Mutex::new(self.workdir),
            gate: Mutex::new(self.gate),
            bound: Mutex::new(std::collections::BTreeMap::new()),
            shift_declaration: AtomicBool::new(false),
            mutate_after_write: AtomicBool::new(false),
            log: Mutex::new(Vec::new()),
        }
    }
}
