use std::sync::Arc;

use anybot_foundation::{BotId, TaskId, TrustedCerts};
use anybot_models::{
    Calibration, Capability, DataPolicy, ModelRef, ProviderEndpoint, ProviderKind, Session,
    ThinkingMode, gateway, resolve, resolve_endpoint,
};
use anybot_protocol::{
    CapState, ComputerCaps, ComputerOwner, ComputerPlatform, ComputerRef, KnownTaskEventKind,
};
use anybot_runner::{BotProfile, TurnModel};
use test_harness::{FakeClock, FakeModel, ModelFormat};

use crate::fake_builder::FakeComputerBuilder;
use crate::fake_computer::FakeComputer;
use crate::mem_store::{MemoryRaw, MemoryRecord};
use crate::mem_watch::MemoryWatch;
use crate::turn::{TurnWorld, WorldSetup};

#[derive(Clone)]
pub struct FallbackTarget {
    pub format: ModelFormat,
    pub port: u16,
    pub model: String,
    pub bot_context: u64,
}

fn apply_world_files(computer: FakeComputerBuilder, setup: &WorldSetup) -> FakeComputerBuilder {
    let mut built = computer;
    for dir in &setup.dirs {
        built = built.directory(dir);
    }
    for (path, target) in &setup.links {
        built = built.link(path, target);
    }
    for path in &setup.unresolvable {
        built = built.unresolvable(path);
    }
    for (path, content) in &setup.files {
        built = built.file(path, content);
    }
    for (line, script) in &setup.scripts {
        built = built.script(line, script.clone());
    }
    let env_refs: Vec<(&str, &str)> = setup
        .inherited_env
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    built.inherited_env(&env_refs)
}

fn setup_caps(setup: &WorldSetup) -> ComputerCaps {
    if let Some(caps) = &setup.caps {
        return caps.clone();
    }
    let owner = setup
        .owner
        .unwrap_or(if setup.platform == ComputerPlatform::BotMachine {
            ComputerOwner::Bot
        } else {
            ComputerOwner::User
        });
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

    pub async fn append_task_cancelled(&self, task: TaskId) -> Result<(), String> {
        let kind = KnownTaskEventKind::TaskCancelled {
            reason: "user".to_owned(),
        };
        self.runner
            .append_task_event(&task, kind)
            .await
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    }

    pub async fn answer_shown_card(
        &self,
        task: TaskId,
        answer: anybot_protocol::CardAnswer,
    ) -> Result<(), String> {
        self.runner
            .append_task_event(
                &task,
                KnownTaskEventKind::CardAnswered {
                    answer,
                    origin: anybot_protocol::AnswerOrigin::Hand,
                },
            )
            .await
            .map(|_| ())
            .map_err(|e| format!("{e:?}"))
    }

    pub fn set_gate(&self, gate: anybot_trust::GateConfig) {
        let computer: &FakeComputer = &self.gated;
        *FakeComputer::guard(&computer.gate) = gate;
    }
}

pub(crate) fn build_world_computer(
    setup: &WorldSetup,
    clock: &Arc<FakeClock>,
) -> FakeComputerBuilder {
    let mut builder = FakeComputerBuilder::default()
        .named(&setup.computer_name)
        .caps(setup_caps(setup))
        .workdir(&setup.workdir)
        .case_insensitive(setup.case_insensitive)
        .gate(setup.gate.clone())
        .ripgrep(setup.ripgrep)
        .clock(Arc::clone(clock));
    if !setup.home_dir.is_empty() {
        builder = builder.home_dir(&setup.home_dir);
    }
    if !setup.anybot_dir.is_empty() {
        builder = builder.anybot_dir(&setup.anybot_dir);
    }
    apply_world_files(builder, setup)
}

pub(crate) fn make_record(
    setup: &WorldSetup,
    model: &Arc<FakeModel>,
    watch: &Arc<MemoryWatch>,
) -> Arc<MemoryRecord> {
    let base = match setup.fail_on_kind {
        Some(kind) => MemoryRecord::fail_on_kind(kind),
        None => MemoryRecord::new(),
    };
    let base = base.with_watch(Arc::clone(watch));
    if !setup.probe_requests {
        return Arc::new(base);
    }
    let watched = Arc::clone(model);
    Arc::new(base.with_probe(Arc::new(move || watched.recorded().len())))
}

pub(crate) fn share_raw(raw: &Arc<MemoryRaw>) -> Arc<dyn anybot_runner::RawStore> {
    let shared: Arc<MemoryRaw> = Arc::clone(raw);
    shared
}

#[must_use]
pub fn bot_profile(setup: &WorldSetup, bot: BotId) -> BotProfile {
    BotProfile {
        id: bot,
        name: setup.bot_name.clone(),
        instructions: setup.instructions.clone(),
        timezone: "Asia/Taipei".to_owned(),
        budget_overrides: setup.budget_overrides.clone(),
    }
}

#[must_use]
pub fn computer_of(setup: &WorldSetup) -> ComputerRef {
    ComputerRef {
        name: setup.computer_name.clone(),
        platform: setup.caps.as_ref().map_or(setup.platform, |c| c.platform),
    }
}

#[must_use]
pub fn provider_kind(format: ModelFormat) -> ProviderKind {
    match format {
        ModelFormat::OpenAi | ModelFormat::Vllm => ProviderKind::OpenAi,
        ModelFormat::Anthropic => ProviderKind::Anthropic,
    }
}

#[must_use]
pub fn model_name_of_setup(format: ModelFormat) -> &'static str {
    match format {
        ModelFormat::OpenAi | ModelFormat::Vllm => "gpt-5",
        ModelFormat::Anthropic => "claude-opus-5-5",
    }
}

#[must_use]
pub fn turn_model(setup: &WorldSetup, port: u16) -> TurnModel {
    let kind = provider_kind(setup.format);
    let name = setup
        .primary_name
        .as_deref()
        .unwrap_or_else(|| model_name_of_setup(setup.format));
    let actual_port = setup.primary_port_override.unwrap_or(port);
    let endpoint = local_endpoint(kind, setup.format, actual_port, name);
    let fallbacks = setup
        .fallbacks
        .iter()
        .map(|target| {
            let fb_kind = provider_kind(target.format);
            ModelRef {
                endpoint: local_endpoint(fb_kind, target.format, target.port, &target.model),
                bot_context: Some(target.bot_context),
            }
        })
        .collect();
    TurnModel {
        primary: ModelRef {
            endpoint,
            bot_context: Some(setup.bot_context),
        },
        fallbacks,
        thinking: ThinkingMode::Auto,
        capability: capability_of(kind, name, setup.bot_context),
        calibration: Calibration::new(),
        reserved_output: setup.reserved_output,
    }
}

#[must_use]
pub fn capability_of(kind: ProviderKind, name: &str, bot_context: u64) -> Capability {
    resolve(kind, name, Some(bot_context), None)
}

#[must_use]
pub fn local_endpoint(
    kind: ProviderKind,
    format: ModelFormat,
    port: u16,
    name: &str,
) -> ProviderEndpoint {
    let wire = wire_format(format);
    resolve_endpoint(
        kind,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        name,
        Some(wire),
    )
}

#[must_use]
pub fn wire_format(format: ModelFormat) -> anybot_models::WireFormat {
    match format {
        ModelFormat::OpenAi | ModelFormat::Vllm => anybot_models::WireFormat::OpenAiCompatible,
        ModelFormat::Anthropic => anybot_models::WireFormat::Anthropic,
    }
}

pub fn build_session(clock: &Arc<FakeClock>) -> Result<Session, String> {
    Ok(Session::new(
        gateway(share_clock(clock), &TrustedCerts::empty())?,
        DataPolicy::LocalOnly,
        share_clock(clock),
        0,
    ))
}

#[must_use]
pub fn share_clock(clock: &Arc<FakeClock>) -> Arc<dyn anybot_foundation::Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}
