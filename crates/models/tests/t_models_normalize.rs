use rustls as _;
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
use test_harness::{ModelFormat, ToolCallScript, model_calls_tool, model_think_then_reply};
#[path = "../test_support/mod.rs"]
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
            .complete(
                &endpoint,
                &asked,
                stream,
                &permit_support::permit_for(&endpoint)?,
            )
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
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                true,
                &permit_support::permit_for(&endpoint)?,
            )
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
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                stream,
                &permit_support::permit_for(&endpoint)?,
            )
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
