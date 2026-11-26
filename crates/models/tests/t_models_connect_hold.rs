use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls as _;
use trybuild as _;
use url as _;

use anybot_foundation::clock::Clock;
use anybot_models::{ProviderKind, ThinkingMode};
use test_harness::{FakeClock, timeout_real};

#[path = "../test_support/ask.rs"]
mod support;

#[tokio::test]
async fn accepted_connection_without_headers_is_first_token() -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|err| format!("{err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("{err}"))?
        .port();
    tokio::spawn(async move { hold_open(listener).await });
    let clock = FakeClock::new(0);
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
    let ready = timeout_real(wait_clock(&clock, &task, 121), Duration::from_secs(30)).await;
    assert!(ready.is_ok(), "still waiting for headers");
    let err = task.await.map_err(|err| format!("{err}"))?.unwrap_err();
    assert!(
        matches!(
            err,
            anybot_models::GatewayFail::Classified { ref classified, .. }
                if classified.code == "model-first-token"
        ),
        "{err:?}"
    );
    Ok(())
}

#[tokio::test]
async fn https_tcp_accept_without_handshake_is_unreachable() -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|err| format!("{err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("{err}"))?
        .port();
    tokio::spawn(async move {
        loop {
            let accepted = listener.accept().await;
            if accepted.is_err() {
                break;
            }
        }
    });
    let clock = FakeClock::new(0);
    let gateway =
        anybot_models::gateway(as_clock(&clock), &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        &format!("https://127.0.0.1:{port}/v1"),
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
    let ready = timeout_real(wait_once(&clock, &task, 16), Duration::from_secs(30)).await;
    assert!(ready.is_ok(), "https handshake still waiting");
    let err = task.await.map_err(|err| format!("{err}"))?.unwrap_err();
    assert!(
        matches!(
            err,
            anybot_models::GatewayFail::Classified { ref classified, .. }
                if classified.code == "model-unreachable"
        ),
        "{err:?}"
    );
    Ok(())
}

async fn hold_open(listener: tokio::net::TcpListener) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        tokio::spawn(async move {
            let _held = stream;
            std::future::pending::<()>().await;
        });
    }
}

async fn wait_once(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::TurnResult, anybot_models::GatewayFail>>,
    secs: u64,
) {
    test_harness::sleep_real(Duration::from_millis(30)).await;
    let _woke = clock.advance(Duration::from_secs(secs));
    loop {
        if task.is_finished() {
            break;
        }
        test_harness::sleep_real(Duration::from_millis(30)).await;
    }
}

async fn wait_clock(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::TurnResult, anybot_models::GatewayFail>>,
    secs: u64,
) {
    loop {
        test_harness::sleep_real(Duration::from_millis(30)).await;
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
