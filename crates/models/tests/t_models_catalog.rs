use proptest as _;
use rustls as _;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls as _;
use trybuild as _;

use anybot_foundation as _;
use anybot_foundation::clock::Clock;
use anybot_models::{
    ListOutcome, ProviderKind, empty_list_code, fetch_models, gateway, normalize_ids,
};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    FakeClock, ModelFormat, model_accept_then_delay, model_list, model_list_silent,
    model_start_with_format,
};
use tiktoken_rs as _;
use tokio as _;
use url as _;

#[test]
fn catalog_sorts_dedupes_and_strips_models_prefix() {
    let names = normalize_ids(&[
        "models/b".to_owned(),
        "a".to_owned(),
        "b".to_owned(),
        "models/a".to_owned(),
    ]);
    assert_eq!(names, vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(empty_list_code(), "model-list-empty");
}

#[tokio::test]
async fn t_models_n3_3() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_list(Vec::new())]);
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let clock_dyn: Arc<dyn Clock> = shared;
    let gateway = gateway(clock_dyn, &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        "gpt-5",
        None,
    );
    let outcome = fetch_models(&gateway, &endpoint)
        .await
        .map_err(|err| format!("{err:?}"))?;
    assert!(matches!(outcome, ListOutcome::Empty), "{outcome:?}");
    assert_eq!(empty_list_code(), "model-list-empty");
    Ok(())
}

#[tokio::test]
async fn t_models_n3_4() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_list_silent()]);
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let gateway = gateway(shared, &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        "gpt-5",
        None,
    );
    let task = tokio::spawn(async move { fetch_models(&gateway, &endpoint).await });
    let ready =
        test_harness::timeout_real(advance_list(&clock, &task), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    let outcome = task.await.unwrap().map_err(|err| format!("{err:?}"))?;
    assert!(matches!(
        outcome,
        ListOutcome::Failed { code, .. } if code == "model-list-timeout"
    ));
    Ok(())
}

#[tokio::test]
async fn list_headers_that_never_arrive_time_out() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    model.script_set(vec![model_accept_then_delay(90, model_list(Vec::new()))]);
    let clock = FakeClock::new(0);
    model.header_clock(Arc::clone(&clock));
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let gateway = gateway(shared, &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        "gpt-5",
        None,
    );
    let task = tokio::spawn(async move { fetch_models(&gateway, &endpoint).await });
    let ready =
        test_harness::timeout_real(advance_list(&clock, &task), Duration::from_secs(5)).await;
    assert!(ready.is_ok(), "list still waiting for headers");
    let outcome = task.await.unwrap().map_err(|err| format!("{err:?}"))?;
    assert!(
        matches!(outcome, ListOutcome::Failed { code, .. } if code == "model-list-timeout"),
        "{outcome:?}"
    );
    Ok(())
}

async fn advance_list(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<ListOutcome, anybot_models::GatewayFail>>,
) {
    loop {
        test_harness::sleep_real(Duration::from_millis(20)).await;
        let _woke = clock.advance(Duration::from_secs(46));
        if task.is_finished() {
            break;
        }
    }
}

#[test]
fn usable_context_takes_the_minimum() {
    let usable = anybot_models::usable_context(200_000, Some(128_000), Some(100_000), 8_000);
    assert_eq!(usable, 92_000);
}

#[test]
fn formula_counts_ascii_and_cjk() {
    let english = anybot_models::formula_tokens(&anybot_models::EstimateInput {
        text: "hello world",
        messages: 1,
        images: 0,
    });
    assert_eq!(english, 3 + 4);
    let cjk = anybot_models::formula_tokens(&anybot_models::EstimateInput {
        text: "\u{4f60}\u{597d}",
        messages: 0,
        images: 1,
    });
    assert_eq!(cjk, 2 + 1_600);
}
