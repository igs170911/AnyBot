use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use tokio as _;
use url as _;

use anybot_models::{ProviderKind, ThinkingMode, resolve_endpoint};
use test_harness::{ModelFormat, model_replies_text};

#[path = "common/mod.rs"]
mod support;

#[tokio::test]
async fn t_models_providers_2() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    world.model.expect_key("k1");
    support::script(&world.model, vec![model_replies_text("ok", false)]);
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{}/v1/", world.port),
        " k1 ",
        "local-model",
        None,
    );
    let result = world
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    assert_eq!(result.text, "ok");
    let recorded = world.model.recorded();
    let first = recorded.first().unwrap();
    assert!(first.path.ends_with("/v1/chat/completions"));
    assert!(!first.path.contains("//chat"));
    assert_eq!(first.key_match, test_harness::KeyMatch::Match);
    Ok(())
}

#[tokio::test]
async fn t_models_providers_3() -> Result<(), String> {
    let claude = support::world(ModelFormat::Anthropic).await?;
    support::script(&claude.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k");
    claude
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    let recorded = claude.model.recorded();
    let body = &recorded.first().unwrap().body;
    assert!(body.contains("\"max_tokens\":64000"));
    let router = support::world(ModelFormat::OpenAi).await?;
    support::script(&router.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(
        ProviderKind::OpenRouter,
        router.port,
        "openrouter/auto",
        "k",
    );
    router
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    let recorded = router.model.recorded();
    let headers = &recorded.first().unwrap().headers;
    let title = headers.iter().find(|(name, _)| name == "x-title");
    assert!(
        matches!(title, Some((_, test_harness::HeaderValueKind::Value(value))) if value == "AnyBot")
    );
    let _clock = router.clock.now_ms();
    Ok(())
}

#[tokio::test]
async fn custom_endpoint_without_key_sends_no_key() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    world.model.expect_key("openai-secret");
    support::script(&world.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::Custom, world.port, "local-model", "");
    world
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    assert_eq!(
        world.model.recorded().first().unwrap().key_match,
        test_harness::KeyMatch::Missing
    );
    Ok(())
}

#[tokio::test]
async fn t_models_thinking_1() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    world
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Deep), false)
        .await
        .unwrap();
    let recorded = world.model.recorded();
    let body = &recorded.first().unwrap().body;
    assert!(body.contains("\"reasoning_effort\":\"high\""));
    Ok(())
}
