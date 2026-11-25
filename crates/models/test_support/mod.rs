use std::sync::Arc;

use anybot_foundation::clock::Clock;
use anybot_models::{
    Gateway, ProviderEndpoint, ProviderKind, WireFormat, gateway, resolve_endpoint,
};
use proptest as _;
use test_harness::{FakeClock, FakeModel, ModelFormat, Step, model_start_with_format};

pub(crate) mod ask;

pub(crate) struct World {
    pub gateway: Gateway,
    pub model: Arc<FakeModel>,
    pub port: u16,
    pub clock: Arc<FakeClock>,
}

pub(crate) async fn world(format: ModelFormat) -> Result<World, String> {
    let (model, port) = model_start_with_format(format).await?;
    let clock = FakeClock::new(0);
    let gateway = gateway(share_clock(&clock))?;
    Ok(World {
        gateway,
        model,
        port,
        clock,
    })
}

pub(crate) fn endpoint(kind: ProviderKind, port: u16, model: &str, key: &str) -> ProviderEndpoint {
    let format = if matches!(kind, ProviderKind::Anthropic) {
        Some(WireFormat::Anthropic)
    } else {
        Some(WireFormat::OpenAiCompatible)
    };
    resolve_endpoint(
        kind,
        &format!("http://127.0.0.1:{port}/v1"),
        key,
        model,
        format,
    )
}

pub(crate) use ask::hi;

fn share_clock(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

pub(crate) fn script(model: &FakeModel, steps: Vec<Step>) {
    model.script_set(steps);
}
