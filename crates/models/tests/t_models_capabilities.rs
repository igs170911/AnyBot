use proptest as _;
use std::sync::Arc;

use anybot_foundation as _;
use anybot_foundation::clock::Clock;
use anybot_models::{
    Calibration, EstimateInput, ProviderKind, apply_calibration, builtin_rows, formula_tokens,
    margin_milli, resolve,
};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{FakeClock, ModelFormat, model_start_with_format};
use tokio as _;
use url as _;

#[test]
fn t_models_capabilities_1() {
    let rows = builtin_rows();
    assert!(rows.len() >= 8);
    for row in &rows {
        assert!(!row.model.is_empty());
        assert!(row.context_tokens > 0);
        assert!(row.max_output_tokens > 0);
        assert!(!row.source.is_empty());
        if matches!(row.provider, ProviderKind::Anthropic) {
            assert!(row.max_output_tokens <= 64_000);
        }
    }
}

#[test]
fn t_models_capabilities_2() {
    let row = resolve(ProviderKind::Custom, "local-model", Some(32_000), None);
    assert_eq!(row.context_tokens, 32_000);
    assert_eq!(row.max_output_tokens, 8_192);
    assert!(row.tools);
    assert!(row.stream);
    assert_eq!(row.tokenizer, anybot_models::TokenizerKind::Formula);
}

#[test]
fn t_models_capabilities_5() {
    let usable = anybot_models::usable_context(200_000, Some(128_000), Some(100_000), 8_000);
    assert_eq!(usable, 92_000);
}

#[test]
fn t_models_capabilities_7() {
    let mut calibration = Calibration::new();
    let estimated = formula_tokens(&EstimateInput {
        text: "hello",
        messages: 1,
        images: 0,
    });
    let actual = estimated.saturating_mul(13) / 10;
    for _ in 0..4 {
        assert_eq!(margin_milli(calibration.samples()), 1_150);
        calibration.record(estimated, actual);
    }
    calibration.record(estimated, actual);
    let adjusted = apply_calibration(estimated, &calibration);
    assert!(adjusted >= actual);
    assert!(adjusted <= actual.saturating_mul(115) / 100);
}

#[tokio::test]
async fn over_budget_request_is_not_sent() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let clock_dyn: Arc<dyn Clock> = shared;
    let gateway = anybot_models::gateway(clock_dyn);
    let mut session = anybot_models::Session::new(gateway, share(&clock), 0);
    let primary = anybot_models::ModelRef {
        endpoint: anybot_models::resolve_endpoint(
            ProviderKind::Custom,
            &format!("http://127.0.0.1:{port}/v1"),
            "",
            "local-model",
            None,
        ),
        bot_context: Some(10_000),
    };
    let mut request = short_hi();
    request.estimate_override = Some(11_000);
    let policy = anybot_models::DataPolicy::LocalOnly;
    let mut plan = anybot_models::ask_plan(
        &primary,
        &[],
        &policy,
        &request,
        anybot_models::Audience::Watching,
    );
    let outcome = session.ask(&mut plan).await;
    assert!(matches!(
        outcome,
        Err(anybot_models::GatewayFail::Classified { classified, .. })
            if classified.code == "model-budget"
    ));
    assert!(model.recorded().is_empty());
    Ok(())
}

fn share(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

fn short_hi() -> anybot_models::TurnRequest {
    anybot_models::TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: anybot_models::Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking: anybot_models::ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}

#[test]
fn t_models_capabilities_9() {
    let mut seed = 7_u64;
    for _ in 0..1_000 {
        seed = seed.wrapping_mul(17).wrapping_add(3);
        let text = format!("sample-{seed}");
        let input = EstimateInput {
            text: &text,
            messages: 1,
            images: 0,
        };
        let first = formula_tokens(&input);
        let second = formula_tokens(&input);
        assert_eq!(first, second);
    }
}
