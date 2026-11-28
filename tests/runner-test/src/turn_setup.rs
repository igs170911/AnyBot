use std::sync::Arc;

use anybot_foundation::{BotId, TrustedCerts};
use anybot_models::{
    Calibration, Capability, DataPolicy, ModelRef, ProviderEndpoint, ProviderKind, Session,
    ThinkingMode, gateway, resolve, resolve_endpoint,
};
use anybot_protocol::{ComputerOwner, ComputerRef};
use anybot_runner::{BotProfile, TurnModel};
use test_harness::{FakeClock, FakeModel, ModelFormat};

use crate::fake_builder::FakeComputerBuilder;
use crate::mem_store::{MemoryRaw, MemoryRecord};
use crate::turn::WorldSetup;

pub(crate) fn build_world_computer(
    setup: &WorldSetup,
    clock: &Arc<FakeClock>,
) -> FakeComputerBuilder {
    let mut builder = FakeComputerBuilder::default()
        .named(&setup.computer_name)
        .platform(setup.platform);
    builder = match setup.owner {
        ComputerOwner::User => builder.user_machine().commands().files(),
        ComputerOwner::Bot | ComputerOwner::Unknown => builder.bot_machine(),
    };
    builder = builder
        .sandbox(setup.sandbox)
        .workdir(&setup.workdir)
        .case_insensitive(setup.case_insensitive)
        .gate(setup.gate.clone())
        .clock(Arc::clone(clock));
    if !setup.home_dir.is_empty() {
        builder = builder.home_dir(&setup.home_dir);
    }
    if !setup.anybot_dir.is_empty() {
        builder = builder.anybot_dir(&setup.anybot_dir);
    }
    apply_world_files(builder, setup)
}

fn apply_world_files(computer: FakeComputerBuilder, setup: &WorldSetup) -> FakeComputerBuilder {
    let mut built = computer;
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
    built
}

pub(crate) fn make_record(setup: &WorldSetup, model: &Arc<FakeModel>) -> Arc<MemoryRecord> {
    let base = match setup.fail_on_kind {
        Some(kind) => MemoryRecord::fail_on_kind(kind),
        None => MemoryRecord::new(),
    };
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
    }
}

#[must_use]
pub fn computer_of(setup: &WorldSetup) -> ComputerRef {
    ComputerRef {
        name: setup.computer_name.clone(),
        platform: setup.platform,
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
    let name = model_name_of_setup(setup.format);
    let endpoint = local_endpoint(kind, setup.format, port, name);
    TurnModel {
        primary: ModelRef {
            endpoint,
            bot_context: Some(setup.bot_context),
        },
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
