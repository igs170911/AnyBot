use proptest as _;
use rustls as _;
use serde as _;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls as _;
use trybuild as _;

use anybot_foundation::clock::Clock;
use anybot_models::{ProviderKind, ThinkingMode};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use test_harness::{
    FakeClock, ModelFormat, model_accept_then_delay, model_replies_text, model_start_with_format,
    sleep_real, timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/ask.rs"]
mod support;

#[tokio::test]
async fn gateway_connect_timeout_uses_clock() -> Result<(), String> {
    let clock = FakeClock::new(0);
    let gateway =
        anybot_models::gateway(as_clock(&clock), &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        "http://192.0.2.1:81/v1",
        "k",
        "gpt-5",
        None,
    );
    let task = tokio::spawn(async move {
        gateway
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                false,
                &anybot_models::SendPermit::for_endpoint(
                    &anybot_models::DataPolicy::Cloud {
                        allowed: vec![ProviderKind::OpenAi],
                    },
                    &endpoint,
                    anybot_models::ModelUse::Bot,
                )
                .ok_or(anybot_models::GatewayFail::MissingKey)?,
            )
            .await
    });
    let ready = timeout_real(wait_clock(&clock, &task, 16), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    let err = task.await.unwrap().unwrap_err();
    assert!(matches!(
        err,
        anybot_models::GatewayFail::Classified { classified, .. }
            if classified.code == "model-unreachable"
    ));
    Ok(())
}

async fn wait_clock(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::TurnResult, anybot_models::GatewayFail>>,
    secs: u64,
) {
    loop {
        sleep_real(Duration::from_millis(30)).await;
        let _woke = clock.advance(Duration::from_secs(secs));
        if task.is_finished() {
            break;
        }
    }
}

fn as_clock(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

#[tokio::test]
async fn header_delay_is_not_a_connect_timeout() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_accept_then_delay(
        20,
        model_replies_text("late", false),
    )]);
    let clock = FakeClock::new(0);
    model.header_clock(Arc::clone(&clock));
    let gateway =
        anybot_models::gateway(as_clock(&clock), &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        "gpt-5",
        None,
    );
    let task = tokio::spawn(async move {
        gateway
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                false,
                &anybot_models::SendPermit::for_endpoint(
                    &anybot_models::DataPolicy::Cloud {
                        allowed: vec![ProviderKind::OpenAi],
                    },
                    &endpoint,
                    anybot_models::ModelUse::Bot,
                )
                .ok_or(anybot_models::GatewayFail::MissingKey)?,
            )
            .await
    });
    let ready = timeout_real(wait_clock(&clock, &task, 20), Duration::from_secs(8)).await;
    assert!(ready.is_ok());
    let result = task
        .await
        .map_err(|err| format!("{err}"))?
        .map_err(|err| format!("{err:?}"))?;
    assert_eq!(result.text, "late");
    assert_eq!(model.recorded().len(), 1);
    Ok(())
}

#[tokio::test]
async fn pinned_local_address_does_not_relax_a_cloud_name() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_replies_text("nope", false)]);
    let clock = FakeClock::new(0);
    let mut gateway =
        anybot_models::gateway(as_clock(&clock), &anybot_foundation::TrustedCerts::empty())?;
    gateway.pin_host("api.cloud.test", SocketAddr::from(([127, 0, 0, 1], port)));
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        "http://api.cloud.test/v1",
        "k",
        "gpt-5",
        None,
    );
    let issued = anybot_models::SendPermit::for_endpoint(
        &anybot_models::DataPolicy::LocalOnly,
        &endpoint,
        anybot_models::ModelUse::Bot,
    );
    assert!(issued.is_none());
    let local = anybot_models::resolve_endpoint(
        ProviderKind::Custom,
        "http://localhost:9/v1",
        "",
        "m",
        None,
    );
    let permit = anybot_models::SendPermit::for_endpoint(
        &anybot_models::DataPolicy::LocalOnly,
        &local,
        anybot_models::ModelUse::Bot,
    )
    .ok_or("local permit")?;
    let mut pinned =
        anybot_models::gateway(as_clock(&clock), &anybot_foundation::TrustedCerts::empty())?;
    pinned.pin_host("localhost", SocketAddr::from(([203, 0, 113, 5], 9)));
    let err = pinned
        .complete(&local, &support::hi(ThinkingMode::Auto), false, &permit)
        .await;
    assert!(err.is_err(), "{err:?}");
    assert!(model.recorded().is_empty());
    Ok(())
}
