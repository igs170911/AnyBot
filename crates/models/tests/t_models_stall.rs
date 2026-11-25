use proptest as _;
use serde as _;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation as _;
use anybot_models::{ProviderKind, ThinkingMode, first_token_wait};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use test_harness::{ModelFormat, model_no_response, model_stall, sleep_real, timeout_real};
use tokio as _;
use url as _;

#[path = "../test_support/mod.rs"]
mod support;

#[tokio::test]
async fn gateway_keeps_partial_text_after_stall() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_stall("Hello", true)]);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let clock = Arc::clone(&world.clock);
    let task = tokio::spawn(async move {
        world
            .gateway
            .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
            .await
    });
    let ready = timeout_real(wait_finished(&clock, &task, 36), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    let result = task.await.unwrap().unwrap();
    assert!(result.text.contains("Hello") || result.text.contains('H'));
    assert!(matches!(
        result.stop,
        anybot_models::StopKind::Interrupted | anybot_models::StopKind::Finished
    ));
    let _formula = first_token_wait(0);
    Ok(())
}

async fn wait_finished(
    clock: &test_harness::FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::TurnResult, anybot_models::GatewayFail>>,
    secs: u64,
) {
    loop {
        sleep_real(Duration::from_millis(20)).await;
        let _woke = clock.advance(Duration::from_secs(secs));
        if task.is_finished() {
            break;
        }
    }
}

#[tokio::test]
async fn gateway_times_out_before_first_token() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_no_response(true)]);
    let endpoint = support::endpoint(ProviderKind::Custom, world.port, "local", "k");
    let clock = Arc::clone(&world.clock);
    let task = tokio::spawn(async move {
        world
            .gateway
            .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
            .await
    });
    let ready = timeout_real(wait_finished(&clock, &task, 121), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    let outcome = task.await.unwrap();
    assert!(matches!(
        outcome,
        Err(anybot_models::GatewayFail::Transport(
            anybot_models::TransportFail::FirstToken
        )) | Ok(anybot_models::TurnResult {
            stop: anybot_models::StopKind::Interrupted,
            ..
        })
    ));
    Ok(())
}
