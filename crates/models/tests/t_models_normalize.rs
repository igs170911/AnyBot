use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use tokio as _;
use url as _;

use anybot_models::{ProviderKind, StopKind, ThinkingMode, ToolArguments};
use test_harness::{
    ModelFormat, ToolCallScript, UsageScript, model_calls_tool, model_stop_reason,
    model_think_then_reply, model_tool_calls, model_tool_calls_cut, model_tool_calls_length,
    model_usage,
};
#[path = "common/mod.rs"]
mod support;

fn with_tool(request: anybot_models::TurnRequest, name: &str) -> anybot_models::TurnRequest {
    let mut next = request;
    next.tools.push(anybot_models::ToolSpec {
        name: name.to_owned(),
        description: "read".to_owned(),
        parameters: serde_json::json!({ "type": "object" }),
    });
    next
}

fn read_file_call() -> ToolCallScript {
    let mut call = ToolCallScript::named("read_file", r#"{"path":"a.txt"}"#);
    call.argument_parts = vec![r#"{"path":"#.to_owned(), r#""a.txt"}"#.to_owned()];
    call
}

#[tokio::test]
async fn t_models_normalize_1() -> Result<(), String> {
    for stream in [false, true] {
        let world = match support::world(ModelFormat::Vllm).await {
            Ok(world) => world,
            Err(err) => return Err(err),
        };
        support::script(
            &world.model,
            vec![model_calls_tool(
                "read_file",
                serde_json::json!({"path":"a.txt"}),
                stream,
            )],
        );
        let endpoint = support::endpoint(ProviderKind::Custom, world.port, "local", "k");
        let asked = with_tool(support::hi(ThinkingMode::Auto), "read_file");
        let result = world
            .gateway
            .complete(&endpoint, &asked, stream)
            .await
            .unwrap();
        assert_eq!(result.stop, StopKind::NeedsTool);
        assert_eq!(result.tool_calls.len(), 1);
        let call = result.tool_calls.first().expect("call");
        assert_eq!(call.name, "read_file");
        assert_eq!(
            call.arguments,
            ToolArguments::Json(serde_json::json!({"path":"a.txt"}))
        );
        assert!(result.attachment.is_none());
        let _now = world.clock.now_ms();
    }
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_2() -> Result<(), String> {
    for format in [ModelFormat::OpenAi, ModelFormat::Vllm] {
        let world = match support::world(format).await {
            Ok(world) => world,
            Err(err) => return Err(err),
        };
        support::script(
            &world.model,
            vec![model_think_then_reply("plan", "ok", true)],
        );
        let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
        let result = world
            .gateway
            .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
            .await
            .unwrap();
        assert_eq!(result.thinking, "plan");
        assert_eq!(result.text, "ok");
        assert_eq!(result.stop, StopKind::Finished);
    }
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_3() -> Result<(), String> {
    for stream in [false, true] {
        let world = match support::world(ModelFormat::Anthropic).await {
            Ok(world) => world,
            Err(err) => return Err(err),
        };
        let call = ToolCallScript::named("read_file", r#"{"path":"a.txt"}"#);
        support::script(
            &world.model,
            vec![test_harness::model_think_then_tool(
                "plan",
                "sig-1",
                vec![call],
                stream,
            )],
        );
        let endpoint =
            support::endpoint(ProviderKind::Anthropic, world.port, "claude-opus-5-5", "k");
        let result = world
            .gateway
            .complete(&endpoint, &support::hi(ThinkingMode::Auto), stream)
            .await
            .unwrap();
        assert_eq!(result.thinking, "plan");
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls.first().expect("call").name, "read_file");
        assert_eq!(result.stop, StopKind::NeedsTool);
        assert_eq!(
            result.attachment.as_ref().map(|item| item.provider),
            Some(ProviderKind::Anthropic)
        );
        assert_eq!(
            result.attachment.as_ref().map(|item| item.model.as_str()),
            Some("claude-opus-5-5")
        );
    }
    Ok(())
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), true)
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
    let result = claude
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
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
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    let usage = match result.usage {
        anybot_models::UsageReport::Present(u) => u,
        anybot_models::UsageReport::Absent => panic!("usage"),
    };
    assert_eq!(usage.input_total(), 6_200);
    assert_eq!(usage.cache_read_tokens, 5_000);
    assert_eq!(usage.output_tokens, 50);
    Ok(())
}
