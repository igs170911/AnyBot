use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::TrustedCerts;
use anybot_foundation::clock::Clock;
use anybot_models::{
    Audience, DataPolicy, FailKind, GatewayFail, ModelRef, ProviderKind, Session, ThinkingMode,
    ask_plan, gateway, resolve_endpoint,
};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness::{
    FakeClock, ModelFormat, advance_clock, model_replies_text, model_reply_status,
    model_start_with_format, sleep_real, timeout_real,
};
use tiktoken_rs as _;
use tokio as _;
use tokio_rustls as _;
use trybuild as _;
use url as _;

#[path = "../test_support/ask.rs"]
mod support;

fn as_clock(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

fn local_ref(port: u16, model: &str, bot_context: u64) -> ModelRef {
    ModelRef {
        endpoint: resolve_endpoint(
            ProviderKind::OpenAi,
            &format!("http://127.0.0.1:{port}/v1"),
            "k",
            model,
            None,
        ),
        bot_context: Some(bot_context),
    }
}

#[tokio::test]
async fn ask_step_returns_fallback_without_calling_it() -> Result<(), String> {
    let (m1, p1) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (m2, p2) = model_start_with_format(ModelFormat::OpenAi).await?;
    m1.script_set(vec![
        model_reply_status(503, "busy"),
        model_reply_status(503, "busy"),
        model_reply_status(503, "busy"),
    ]);
    m2.script_set(vec![model_replies_text("ok", false)]);
    let clock = FakeClock::new(0);
    let gw = gateway(as_clock(&clock), &TrustedCerts::empty())?;
    let mut session = Session::new(gw, DataPolicy::LocalOnly, as_clock(&clock), 0);
    let primary = local_ref(p1, "m1", 128_000);
    let fallbacks = vec![local_ref(p2, "m2", 16_000)];
    let req = support::hi(ThinkingMode::Auto);
    let handle = tokio::spawn(async move {
        let mut plan = ask_plan(&primary, &fallbacks, &req, Audience::Watching);
        session.ask_step(&mut plan).await
    });
    for _step in 0..30 {
        sleep_real(Duration::from_millis(10)).await;
        advance_clock(&clock, Duration::from_secs(1));
        if handle.is_finished() {
            break;
        }
    }
    let res = timeout_real(handle, Duration::from_secs(5))
        .await?
        .map_err(|e| format!("{e}"))?;
    assert_eq!(m1.recorded().len(), 3);
    assert_eq!(m2.recorded().len(), 0);
    let Err(GatewayFail::Exhausted {
        fallback: Some(choice),
        classified,
        ..
    }) = res
    else {
        return Err("expected Exhausted with fallback".to_owned());
    };
    assert_eq!(choice.model.endpoint.model, "m2");
    assert_eq!(choice.reason_code, "model-handoff-fallback");
    assert_eq!(classified.kind, FailKind::Server);
    Ok(())
}

#[tokio::test]
async fn ask_step_non_switchable_stays_classified() -> Result<(), String> {
    let (m1, p1) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (m2, p2) = model_start_with_format(ModelFormat::OpenAi).await?;
    m1.script_set(vec![model_reply_status(404, "missing")]);
    let clock = FakeClock::new(0);
    let gw = gateway(as_clock(&clock), &TrustedCerts::empty())?;
    let mut session = Session::new(gw, DataPolicy::LocalOnly, as_clock(&clock), 0);
    let primary = local_ref(p1, "m1", 128_000);
    let fallbacks = vec![local_ref(p2, "m2", 16_000)];
    let req = support::hi(ThinkingMode::Auto);
    let mut plan = ask_plan(&primary, &fallbacks, &req, Audience::Watching);
    let res = session.ask_step(&mut plan).await;
    assert_eq!(m2.recorded().len(), 0);
    assert!(matches!(
        res,
        Err(GatewayFail::Classified { classified, .. }) if classified.kind == FailKind::NotFound
    ));
    Ok(())
}
