use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use url as _;

use std::sync::Arc;
use std::time::Duration;

use anybot_models::{
    Audience, DataPolicy, ModelRef, ProviderKind, Session, ThinkingMode, ask_plan, gateway,
};
use test_harness::{FakeClock, FakeModel, ModelFormat, Step, model_start_with_format};
use tokio::sync::watch;

pub(crate) struct World {
    pub model: Arc<FakeModel>,
    pub port: u16,
    pub clock: Arc<FakeClock>,
}

pub(crate) struct Pair {
    pub primary: ModelRef,
    pub fallback: ModelRef,
    pub other: Arc<FakeModel>,
}

pub(crate) async fn world(format: ModelFormat) -> Result<World, String> {
    let (model, port) = model_start_with_format(format).await?;
    let clock = FakeClock::new(0);
    Ok(World { model, port, clock })
}

pub(crate) fn endpoint(
    kind: ProviderKind,
    port: u16,
    model: &str,
    key: &str,
) -> anybot_models::ProviderEndpoint {
    let format = if matches!(kind, ProviderKind::Anthropic) {
        Some(anybot_models::WireFormat::Anthropic)
    } else {
        Some(anybot_models::WireFormat::OpenAiCompatible)
    };
    anybot_models::resolve_endpoint(
        kind,
        &format!("http://127.0.0.1:{port}/v1"),
        key,
        model,
        format,
    )
}

pub(crate) fn hi(thinking: ThinkingMode) -> anybot_models::TurnRequest {
    anybot_models::TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: anybot_models::Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}

pub(crate) async fn pair(format: ModelFormat) -> Result<(World, Pair), String> {
    let world = world(format).await?;
    let port = world.port;
    let (other, other_port) = model_start_with_format(format).await?;
    Ok((
        world,
        Pair {
            primary: ModelRef {
                endpoint: endpoint(ProviderKind::OpenAi, port, "m1", "k"),
                bot_context: None,
            },
            fallback: ModelRef {
                endpoint: endpoint(ProviderKind::OpenAi, other_port, "m2", "k"),
                bot_context: None,
            },
            other,
        },
    ))
}

pub(crate) fn session(world: &World) -> Session {
    Session::new(gateway(share(&world.clock)), share(&world.clock), 0)
}

pub(crate) async fn ask_watching(
    session: &mut Session,
    primary: &ModelRef,
    fallbacks: &[ModelRef],
    thinking: ThinkingMode,
) -> Result<anybot_models::AskOutcome, String> {
    let request = hi(thinking);
    let policy = DataPolicy::LocalOnly;
    let mut plan = ask_plan(primary, fallbacks, &policy, &request, Audience::Watching);
    session
        .ask(&mut plan)
        .await
        .map_err(|err| format!("{err:?}"))
}

pub(crate) async fn drive_until(clock: &FakeClock, done: impl Fn() -> bool) {
    loop {
        test_harness::sleep_real(Duration::from_millis(20)).await;
        let _woke = clock.advance(Duration::from_secs(2));
        if done() {
            break;
        }
    }
}

pub(crate) fn stopped() -> (watch::Sender<bool>, watch::Receiver<bool>) {
    watch::channel(false)
}

pub(crate) fn share(clock: &Arc<FakeClock>) -> Arc<dyn anybot_foundation::clock::Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

pub(crate) fn script(model: &FakeModel, steps: Vec<Step>) {
    model.script_set(steps);
}
