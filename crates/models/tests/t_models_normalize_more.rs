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
use tiktoken_rs as _;
use tokio as _;
use url as _;

use anybot_models::{NativeAttachment, ProviderKind, Role, ThinkingMode, ToolArguments, ToolCall};
use test_harness::{ModelFormat, model_omit_usage, model_replies_text};

#[path = "../test_support/mod.rs"]
mod support;

#[tokio::test]
async fn t_models_normalize_12() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_omit_usage("ok")]);
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
    let recorded = world.model.recorded();
    assert!(recorded.first().unwrap().body.contains("include_usage"));
    assert!(matches!(result.usage, anybot_models::UsageReport::Absent));
    Ok(())
}

#[tokio::test]
async fn t_models_normalize_13() -> Result<(), String> {
    let request = anybot_models::TurnRequest {
        system: "be brief".to_owned(),
        messages: vec![
            anybot_models::Message {
                role: Role::User,
                text: "hi".to_owned(),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
            anybot_models::Message {
                role: Role::Assistant,
                text: String::new(),
                tool_calls: vec![ToolCall {
                    id: "functions.read:0".to_owned(),
                    name: "read".to_owned(),
                    arguments: ToolArguments::Json(serde_json::json!({})),
                }],
                tool_call_id: None,
            },
            anybot_models::Message {
                role: Role::Tool,
                text: "file".to_owned(),
                tool_calls: Vec::new(),
                tool_call_id: Some("functions.read:0".to_owned()),
            },
        ],
        tools: vec![anybot_models::ToolSpec {
            name: "read".to_owned(),
            description: "read".to_owned(),
            parameters: serde_json::json!({"type":"object"}),
        }],
        thinking: ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: true,
        estimate_override: None,
    };
    let openai = support::world(ModelFormat::OpenAi).await?;
    support::script(&openai.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::OpenAi, openai.port, "gpt-5", "k");
    openai
        .gateway
        .complete(
            &endpoint,
            &request,
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    let recorded = openai.model.recorded();
    let body = &recorded.first().unwrap().body;
    assert!(body.contains("\"role\":\"system\""));
    assert!(body.contains("\"role\":\"tool\""));
    claude_roundtrip(&request).await
}

async fn claude_roundtrip(request: &anybot_models::TurnRequest) -> Result<(), String> {
    let claude = support::world(ModelFormat::Anthropic).await?;
    support::script(&claude.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::Anthropic, claude.port, "claude-opus-5-5", "k");
    claude
        .gateway
        .complete(
            &endpoint,
            request,
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .map_err(|err| format!("{err:?}"))?;
    let recorded = claude.model.recorded();
    let body = &recorded.first().ok_or("claude body")?.body;
    assert!(body.contains("cache_control"));
    assert!(body.contains("tool_use_id"));
    assert!(body.contains("functions_read_0"));
    assert!(!body.contains("functions.read:0"));
    Ok(())
}

async fn send_native(
    world: &support::World,
    endpoint: &anybot_models::ProviderEndpoint,
    request: &anybot_models::TurnRequest,
) -> Result<(), String> {
    world
        .gateway
        .complete(
            endpoint,
            request,
            false,
            &permit_support::permit_for(endpoint)?,
        )
        .await
        .map(|_| ())
        .map_err(|err| format!("{err:?}"))
}

#[tokio::test]
async fn gateway_sends_native_block_only_to_same_model() -> Result<(), String> {
    let attachment = NativeAttachment {
        provider: ProviderKind::Anthropic,
        model: "claude-opus-5-5".to_owned(),
        bytes: serde_json::json!({"type":"thinking","thinking":"plan","signature":"sig-1"})
            .to_string()
            .into_bytes(),
    };
    let mut request = support::hi(ThinkingMode::Auto);
    request.messages.insert(
        0,
        anybot_models::Message {
            role: Role::Assistant,
            text: "earlier".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    );
    request.messages.insert(
        0,
        anybot_models::Message {
            role: Role::User,
            text: "before".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    );
    request.attachments.push(attachment);
    let same = support::world(ModelFormat::Anthropic).await?;
    support::script(&same.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::Anthropic, same.port, "claude-opus-5-5", "k");
    send_native(&same, &endpoint, &request).await?;
    let recorded = same.model.recorded();
    assert!(recorded.first().unwrap().body.contains("sig-1"));
    let other = support::world(ModelFormat::Anthropic).await?;
    support::script(&other.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::Anthropic, other.port, "claude-haiku-5-5", "k");
    send_native(&other, &endpoint, &request).await?;
    let recorded = other.model.recorded();
    assert!(!recorded.first().unwrap().body.contains("sig-1"));
    let openai = support::world(ModelFormat::OpenAi).await?;
    support::script(&openai.model, vec![model_replies_text("ok", false)]);
    let endpoint = support::endpoint(ProviderKind::OpenAi, openai.port, "gpt-5", "k");
    openai
        .gateway
        .complete(
            &endpoint,
            &request,
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .unwrap();
    let recorded = openai.model.recorded();
    assert!(!recorded.first().unwrap().body.contains("sig-1"));
    let _now = openai.clock.now_ms();
    Ok(())
}
