use std::sync::Arc;

use anybot_foundation::{BotId, DeviceId, TaskId};
use anybot_messages::Lang;
use anybot_models::ToolSpec;
use anybot_protocol::{
    ComputerPlatform, KnownTaskEventKind, TaskDetail, TaskEvent, TaskEventKind, TaskStatus,
    TurnInitiator,
};
use anybot_runner::{ContinueTurn, RunnerParts, TurnError, TurnOutcome, TurnRunner, UserMessage};
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
    pub tools: Vec<ToolSpec>,
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
            tools: vec![read_file_spec()],
            probe_requests: false,
            fail_on_kind: None,
            sandbox: true,
            files: Vec::new(),
            scripts: Vec::new(),
            gate: anybot_trust::GateConfig::plain(),
        }
    }
}

#[must_use]
pub fn read_file_spec() -> ToolSpec {
    ToolSpec {
        name: "read_file".to_owned(),
        description: "Read a file".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } }
        }),
    }
}

#[must_use]
pub fn tool_spec(name: &str, description: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } }
        }),
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
            .named("bot-computer")
            .bot_machine()
            .sandbox(setup.sandbox)
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
            tools: setup.tools.clone(),
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

    pub async fn continue_task(
        &self,
        task: TaskId,
        initiator: TurnInitiator,
        opening: Option<&str>,
    ) -> Result<TurnOutcome, TurnError> {
        self.runner
            .continue_task(ContinueTurn {
                task,
                initiator,
                opening: opening.map(ToOwned::to_owned),
            })
            .await
    }

    pub async fn continue_from(
        &self,
        task: TaskId,
        from: Device,
        opening: Option<&str>,
    ) -> Result<TurnOutcome, TurnError> {
        self.continue_task(
            task,
            TurnInitiator::You {
                device: self.device(from),
            },
            opening,
        )
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

use crate::turn_setup::{apply_world_files, make_record, share_raw};
pub use crate::turn_setup::{
    bot_profile, build_session, capability_of, computer_of, local_endpoint, model_name_of_setup,
    provider_kind, share_clock, turn_model, wire_format,
};
