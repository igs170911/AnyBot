use anybot_foundation as _;
use anybot_models::{ProviderKind, Role, StopKind, ThinkingMode};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{ModelFormat, model_replies_text, model_thinking_block};
use tokio as _;
use url as _;

#[path = "common/mod.rs"]
mod support;

#[tokio::test]
async fn t_models_normalize_9() -> Result<(), String> {
    let world = support::world(ModelFormat::Anthropic).await?;
    support::script(
        &world.model,
        vec![model_thinking_block("plan", "sig-keep", "ok", false)],
    );
    let endpoint = support::endpoint(ProviderKind::Anthropic, world.port, "claude-opus-5-5", "k");
    let first = world
        .gateway
        .complete(&endpoint, &support::hi(ThinkingMode::Auto), false)
        .await
        .unwrap();
    let attachment = first.attachment.expect("native");
    let raw = attachment.bytes.clone();
    let mut again = support::hi(ThinkingMode::Auto);
    again.messages.insert(
        0,
        anybot_models::Message {
            role: Role::User,
            text: "before".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    );
    again.messages.insert(
        1,
        anybot_models::Message {
            role: Role::Assistant,
            text: "ok".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    );
    again.attachments.push(attachment);
    support::script(&world.model, vec![model_replies_text("next", false)]);
    world
        .gateway
        .complete(&endpoint, &again, false)
        .await
        .unwrap();
    let same = world.model.recorded();
    let body = &same.last().unwrap().body;
    let parsed = serde_json::from_str::<serde_json::Value>(body).unwrap();
    let sent = parsed
        .get("messages")
        .and_then(|v| v.get(1))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.get(0))
        .expect("block")
        .to_string()
        .into_bytes();
    assert_eq!(sent, raw);

    reject_elsewhere(&again).await?;
    let _asked = support::hi(ThinkingMode::Auto);
    Ok(())
}

async fn reject_elsewhere(again: &anybot_models::TurnRequest) -> Result<(), String> {
    let other = support::world(ModelFormat::Anthropic).await?;
    support::script(&other.model, vec![model_replies_text("next", false)]);
    let other_endpoint =
        support::endpoint(ProviderKind::Anthropic, other.port, "claude-haiku-5-5", "k");
    other
        .gateway
        .complete(&other_endpoint, again, false)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let other_body = other
        .model
        .recorded()
        .first()
        .map(|item| item.body.clone())
        .ok_or("other body")?;
    assert!(!other_body.contains("sig-keep"));

    let openai = support::world(ModelFormat::OpenAi).await?;
    support::script(&openai.model, vec![model_replies_text("next", false)]);
    let openai_endpoint = support::endpoint(ProviderKind::OpenAi, openai.port, "gpt-5", "k");
    openai
        .gateway
        .complete(&openai_endpoint, again, false)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let openai_body = openai
        .model
        .recorded()
        .first()
        .map(|item| item.body.clone())
        .ok_or("openai body")?;
    assert!(!openai_body.contains("sig-keep"));
    Ok(())
}

#[tokio::test]
async fn gateway_keeps_tool_stop_when_stream_and_json_agree() -> Result<(), String> {
    let mut seed = 0x9e37_79b9_u64;
    let mut steps = Vec::new();
    let mut expect = Vec::new();
    for index in 0..1_000 {
        seed = seed.wrapping_mul(0x6c07_8965).wrapping_add(index);
        let (step, expect_tool, stream) = test_harness::random_reply(seed);
        steps.push(step);
        expect.push((expect_tool, stream));
    }
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(&world.model, steps);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "model-a", "k");
    for (expect_tool, stream) in expect {
        let result = world
            .gateway
            .complete(&endpoint, &support::hi(ThinkingMode::Auto), stream)
            .await
            .unwrap();
        if expect_tool {
            assert_eq!(result.stop, StopKind::NeedsTool);
        }
        if matches!(result.stop, StopKind::Interrupted) {
            assert!(result.tool_calls.is_empty());
        }
    }
    let _now = world.clock.now_ms();
    Ok(())
}
