use std::sync::Arc;

use anybot_foundation::{BotId, DeviceId, TaskId};
use anybot_messages::Lang;
use anybot_protocol::{
    CapState, ComputerCaps, ComputerOwner, ComputerPlatform, KnownTaskEventKind, TaskDetail,
    TaskEvent, TaskEventKind, TaskStatus,
};
use anybot_runner::{RunnerParts, TurnError, TurnOutcome, TurnRunner, UserMessage};
use test_harness::{
    FakeClock, FakeModel, ModelFormat, RecordedRequest, Step, model_start_with_format,
};

use crate::fake_computer::FakeComputer;
use crate::mem_store::{MemoryRaw, MemoryRecord};

#[derive(Clone)]
pub struct WorldSetup {
    pub format: ModelFormat,
    pub lang: Option<Lang>,
    pub bot_context: u64,
    pub reserved_output: u64,
    pub instructions: String,
    pub platform: ComputerPlatform,
    pub computer_name: String,
    pub caps: Option<ComputerCaps>,
    pub probe_requests: bool,
    pub fail_on_kind: Option<&'static str>,
    pub sandbox: bool,
    pub files: Vec<(String, String)>,
    pub scripts: Vec<(String, crate::fake_scripts::CommandScript)>,
    pub gate: anybot_trust::GateConfig,
}

impl Default for WorldSetup {
    fn default() -> Self {
        WorldSetup {
            format: ModelFormat::OpenAi,
            lang: None,
            bot_context: 32_000,
            reserved_output: 1_024,
            instructions: "Be brief.".to_owned(),
            platform: ComputerPlatform::BotMachine,
            computer_name: "bot-computer".to_owned(),
            caps: None,
            probe_requests: false,
            fail_on_kind: None,
            sandbox: true,
            files: Vec::new(),
            scripts: Vec::new(),
            gate: anybot_trust::GateConfig::plain(),
        }
    }
}

fn setup_caps(setup: &WorldSetup) -> ComputerCaps {
    if let Some(caps) = &setup.caps {
        return caps.clone();
    }
    let owner = if setup.platform == ComputerPlatform::BotMachine {
        ComputerOwner::Bot
    } else {
        ComputerOwner::User
    };
    let sandbox = setup.sandbox && setup.platform != ComputerPlatform::Windows;
    ComputerCaps {
        run_commands: CapState::Supported,
        read_files: CapState::Supported,
        write_files: CapState::Supported,
        check_path: CapState::Supported,
        command_sandbox: CapState::of(sandbox),
        ..ComputerCaps::declared(setup.platform, owner)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Phone,
    Desktop,
}

pub struct TurnWorld {
    pub gated: Arc<FakeComputer>,
    pub runner: Arc<TurnRunner>,
    pub model: Arc<FakeModel>,
    pub port: u16,
    pub clock: Arc<FakeClock>,
    pub record: Arc<MemoryRecord>,
    pub raw: Arc<MemoryRaw>,
    pub bot: BotId,
    pub setup: WorldSetup,
    phone: DeviceId,
    desktop: DeviceId,
}

impl TurnWorld {
    pub async fn start(setup: &WorldSetup) -> Result<Self, String> {
        let (model, port) = model_start_with_format(setup.format).await?;
        let clock = Arc::new(FakeClock::new(1_700_000_000_000));
        let record = make_record(setup, &model);
        let raw = Arc::new(MemoryRaw::new());
        let session = build_session(&clock)?;
        let clock_dyn = share_clock(&clock);
        let bot = BotId::generate(clock_dyn.as_ref());
        let builder = crate::fake_builder::FakeComputerBuilder::default()
            .named(&setup.computer_name)
            .caps(setup_caps(setup))
            .gate(setup.gate.clone())
            .clock(Arc::clone(&clock));
        let builder = apply_world_files(builder, setup);
        let built = Arc::new(builder.build());
        let gated = Arc::clone(&built);
        let gated_dyn: Arc<dyn anybot_trust::GatedComputer> = built;
        let runner = Arc::new(TurnRunner::new(RunnerParts {
            gated: gated_dyn,
            record: crate::mem_store::share_record(&record),
            raw: share_raw(&raw),
            session,
            clock: share_clock(&clock),
            bot: bot_profile(setup, bot),
            model: turn_model(setup, port),
            lang: setup.lang,
            computer: computer_of(setup),
        }));
        Ok(Self {
            gated,
            runner,
            model,
            port,
            clock: Arc::clone(&clock),
            record,
            raw,
            bot,
            setup: setup.clone(),
            phone: DeviceId::generate(clock_dyn.as_ref()),
            desktop: DeviceId::generate(clock_dyn.as_ref()),
        })
    }
}

fn apply_world_files(
    computer: crate::fake_builder::FakeComputerBuilder,
    setup: &WorldSetup,
) -> crate::fake_builder::FakeComputerBuilder {
    let mut built = computer;
    for (path, content) in &setup.files {
        built = built.file(path, content);
    }
    for (line, script) in &setup.scripts {
        built = built.script(line, script.clone());
    }
    built
}

impl TurnWorld {
    #[must_use]
    pub fn runner_gated(&self) -> Arc<dyn anybot_trust::GatedComputer> {
        self.runner.gated_ref()
    }

    #[must_use]
    pub fn runner_gate_config(&self) -> anybot_trust::GateConfig {
        self.runner.gated_ref().gate_config().clone()
    }

    #[must_use]
    pub fn gate_config(&self) -> anybot_trust::GateConfig {
        self.gated.gate_config().clone()
    }

    #[must_use]
    pub fn clock(&self) -> Arc<FakeClock> {
        Arc::clone(&self.clock)
    }

    #[must_use]
    pub fn tree_content(&self, path: &str) -> Option<String> {
        self.gated.file_content(path)
    }

    pub async fn append_task_stopped(&self, task: TaskId) -> Result<(), String> {
        let kind = KnownTaskEventKind::TaskStopped;
        self.runner
            .append_task_event(&task, kind)
            .await
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    }

    pub fn set_gate(&self, gate: anybot_trust::GateConfig) {
        let computer: &FakeComputer = &self.gated;
        *FakeComputer::guard(&computer.gate) = gate;
    }
}

impl TurnWorld {
    pub fn script(&self, steps: Vec<Step>) {
        self.model.script_set(steps);
    }

    pub async fn send(&self, from: Device, text: &str) -> Result<TurnOutcome, TurnError> {
        self.runner
            .on_user_message(UserMessage {
                device: self.device(from),
                text: text.to_owned(),
            })
            .await
    }

    #[must_use]
    pub fn device(&self, which: Device) -> DeviceId {
        match which {
            Device::Phone => self.phone,
            Device::Desktop => self.desktop,
        }
    }

    #[must_use]
    pub fn task_events(&self, task: TaskId) -> Vec<TaskEvent> {
        self.record.task_events_of(task)
    }

    #[must_use]
    pub fn kind_names(&self, task: TaskId) -> Vec<String> {
        self.kinds(task)
            .iter()
            .map(KnownTaskEventKind::kind_name)
            .collect()
    }

    #[must_use]
    pub fn kinds(&self, task: TaskId) -> Vec<KnownTaskEventKind> {
        let mut kinds: Vec<KnownTaskEventKind> = Vec::new();
        for event in self.task_events(task) {
            let TaskEventKind::Known(kind) = event.kind else {
                continue;
            };
            kinds.push(kind);
        }
        kinds
    }

    pub fn task_state(&self, task: TaskId) -> Result<(TaskStatus, Option<TaskDetail>), String> {
        let view = anybot_domain::replay(&self.record.task_events_of(task))
            .map_err(|e| format!("{e:?}"))?;
        Ok((view.status, view.detail))
    }

    #[must_use]
    pub fn chat_requests(&self) -> Vec<RecordedRequest> {
        self.model.recorded()
    }

    #[must_use]
    pub fn runner(&self) -> &Arc<TurnRunner> {
        &self.runner
    }

    #[must_use]
    pub fn task_of(outcome: &TurnOutcome) -> TaskId {
        outcome.task
    }
}

#[must_use]
pub fn share(world: &Arc<TurnWorld>) -> Arc<TurnWorld> {
    Arc::clone(world)
}

#[must_use]
pub fn arc_world(world: TurnWorld) -> Arc<TurnWorld> {
    Arc::new(world)
}

use crate::turn_setup::share_clock;
pub use crate::turn_setup::{
    bot_profile, build_session, capability_of, computer_of, local_endpoint, model_name_of_setup,
    provider_kind, turn_model, wire_format,
};
use crate::turn_setup::{make_record, share_raw};
