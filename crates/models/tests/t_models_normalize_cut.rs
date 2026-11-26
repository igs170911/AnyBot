use rustls as _;
use tokio_rustls as _;
use trybuild as _;
#[path = "../test_support/permit.rs"]
mod permit_support;
use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use tokio as _;
use url as _;

use anybot_models::{ProviderKind, StopKind, ThinkingMode, ToolArguments};
use test_harness::{
    ModelFormat, ToolCallScript, UsageScript, model_stop_reason, model_tool_calls,
    model_tool_calls_cut, model_tool_calls_length, model_usage,
};
#[path = "../test_support/mod.rs"]
mod support;

fn read_file_call() -> ToolCallScript {
    let mut call = ToolCallScript::named("read_file", r#"{"path":"a.txt"}"#);
    call.argument_parts = vec![r#"{"path":"#.to_owned(), r#""a.txt"}"#.to_owned()];
    call
}

#[tokio::test]
async fn t_models_normalize_4() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut call = ToolCallScript::named("read_file", r#"{"path":"a.txt"}"#);
    call.argument_parts = vec![r#"{"path":"#.to_owned(), r#""a.txt"}"#.to_owned()];
    support::script(&world.model, vec![model_tool_calls_cut(vec![call], true)]);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let result = world
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            true,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    assert_eq!(result.stop, StopKind::Interrupted);
    assert!(result.tool_calls.is_empty());
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_6() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut call = ToolCallScript::named("read_file", r#"{"path":"a.txt"}"#);
    call.argument_parts = vec![r#"{"path":"#.to_owned(), r#""a.txt"}"#.to_owned()];
    support::script(
        &world.model,
        vec![model_tool_calls_length(vec![call], true)],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let result = world
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            true,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    assert_eq!(result.stop, StopKind::Length);
    assert!(result.tool_calls.is_empty());
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_5() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![model_stop_reason("done", "tool_calls", false)],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let result = world
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    assert_eq!(result.stop, StopKind::Finished);
    assert_eq!(result.text, "done");
    assert!(result.tool_calls.is_empty());
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_7() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut call = ToolCallScript::named("read_file", r#"{"path":"#);
    call.argument_parts = vec![r#"{"path":"#.to_owned()];
    support::script(
        &world.model,
        vec![model_tool_calls(vec![call], "tool_calls", false)],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let result = world
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(
        result.tool_calls.first().expect("call").arguments,
        ToolArguments::Broken(r#"{"path":"#.to_owned())
    );
    assert_eq!(world.model.recorded().len(), 1);
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_8() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut first = read_file_call();
    first.id = None;
    let second = ToolCallScript::named("echo", r#"{"x":1}"#);
    support::script(
        &world.model,
        vec![model_tool_calls(vec![first, second], "stop", true)],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let result = world
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            true,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    assert_eq!(result.tool_calls.len(), 2);
    let first = result.tool_calls.first().expect("first");
    let second = result.tool_calls.get(1).expect("second");
    assert_eq!(first.name, "read_file");
    assert_eq!(second.name, "echo");
    assert_ne!(first.id, second.id);
    Ok(())
}

async fn complete_hi(
    world: &support::World,
    endpoint: &anybot_models::ProviderEndpoint,
) -> Result<anybot_models::TurnResult, String> {
    world
        .gateway
        .complete(
            endpoint,
            &support::hi(ThinkingMode::Auto),
            false,
            &permit_support::permit_for(endpoint)?,
        )
        .await
        .map_err(|err| format!("{err:?}"))
}

#[tokio::test]
async fn t_models_normalize_11() -> Result<(), String> {
    let claude = support::world(ModelFormat::Anthropic).await?;
    support::script(
        &claude.model,
        vec![model_usage(
            "ok",
            UsageScript {
                input_tokens: 1_000,
                cache_read_tokens: 5_000,
                cache_write_tokens: 200,
                output_tokens: 50,
                thinking_tokens: 0,
            },
            false,
        )],
    );
    let endpoint = support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k");
    let result = complete_hi(&claude, &endpoint).await?;
    let usage = match result.usage {
        anybot_models::UsageReport::Present(u) => u,
        anybot_models::UsageReport::Absent => panic!("usage"),
    };
    assert_eq!(usage.input_total(), 6_200);
    assert_eq!(usage.cache_read_tokens, 5_000);
    assert_eq!(usage.output_tokens, 50);
    let openai = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &openai.model,
        vec![model_usage(
            "ok",
            UsageScript {
                input_tokens: 6_200,
                cache_read_tokens: 5_000,
                cache_write_tokens: 0,
                output_tokens: 50,
                thinking_tokens: 0,
            },
            false,
        )],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, openai.port, "gpt-5", "k");
    let result = openai
        .gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    let usage = match result.usage {
        anybot_models::UsageReport::Present(u) => u,
        anybot_models::UsageReport::Absent => panic!("usage"),
    };
    assert_eq!(usage.input_total(), 6_200);
    assert_eq!(usage.cache_read_tokens, 5_000);
    assert_eq!(usage.output_tokens, 50);
    let _now = openai.clock.now_ms();
    Ok(())
}
