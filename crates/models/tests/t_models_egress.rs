use proptest as _;
use rustls as _;
use std::sync::Arc;
use trybuild as _;

use anybot_foundation as _;
use anybot_foundation::clock::Clock;
use anybot_models::{
    DataPolicy, JudgeCandidate, ModelUse, ProviderKind, RunPlace, allowed, default_policy,
    pick_judge,
};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{FakeClock, ModelFormat, model_redirect, model_start_with_format};
use tokio as _;
use url as _;

#[test]
fn t_models_egress_1() {
    let lan = default_policy("http://192.168.1.20:11434", ProviderKind::Ollama);
    let cloud = default_policy("https://api.openai.com/v1", ProviderKind::OpenAi);
    let gpu = default_policy("http://gpu:8000", ProviderKind::Custom);
    assert_eq!(lan, DataPolicy::LocalOnly);
    assert_eq!(
        cloud,
        DataPolicy::Cloud {
            allowed: vec![ProviderKind::OpenAi]
        }
    );
    assert_eq!(gpu, DataPolicy::LocalOnly);
}

#[test]
fn t_models_egress_5() {
    let policy = DataPolicy::Cloud {
        allowed: vec![ProviderKind::OpenAi, ProviderKind::Anthropic],
    };
    let candidates = [
        JudgeCandidate {
            provider: ProviderKind::OpenAi,
            has_key: true,
            place: RunPlace::Cloud,
        },
        JudgeCandidate {
            provider: ProviderKind::Anthropic,
            has_key: true,
            place: RunPlace::Cloud,
        },
        JudgeCandidate {
            provider: ProviderKind::Ollama,
            has_key: true,
            place: RunPlace::Local,
        },
    ];
    let picked = pick_judge(&policy, ProviderKind::OpenAi, &candidates);
    assert_eq!(
        picked.map(|item| item.provider),
        Some(ProviderKind::Anthropic)
    );
    let only_bot = pick_judge(
        &DataPolicy::LocalOnly,
        ProviderKind::Ollama,
        &candidates[2..],
    );
    assert_eq!(
        only_bot.map(|item| item.provider),
        Some(ProviderKind::Ollama)
    );
    assert!(!allowed(
        &DataPolicy::LocalOnly,
        "https://api.openai.com/v1",
        ProviderKind::OpenAi,
        ModelUse::Bot
    ));
    assert!(!allowed(
        &policy,
        "https://api.openai.com/v1",
        ProviderKind::OpenAi,
        ModelUse::Summary
    ));
}

#[tokio::test]
async fn redirect_is_not_followed() -> Result<(), String> {
    let (local, local_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (cloud, _cloud_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    local.script_set(vec![model_redirect(
        "http://api.cloud.test/v1/chat/completions",
    )]);
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let clock_dyn: Arc<dyn Clock> = shared;
    let gateway = anybot_models::gateway(clock_dyn, &anybot_foundation::TrustedCerts::empty())?;
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{local_port}/v1"),
        "",
        "local",
        None,
    );
    let request = empty_hi();
    let permit =
        anybot_models::SendPermit::for_endpoint(&DataPolicy::LocalOnly, &endpoint, ModelUse::Bot)
            .expect("permit");
    let err = gateway.complete(&endpoint, &request, false, &permit).await;
    assert!(err.is_err());
    assert!(cloud.recorded().is_empty());
    Ok(())
}

#[tokio::test]
async fn local_permit_rejects_cloud_destination() -> Result<(), String> {
    let (cloud, _cloud_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let gateway = anybot_models::gateway(shared, &anybot_foundation::TrustedCerts::empty())?;
    let local = anybot_models::resolve_endpoint(
        ProviderKind::Custom,
        "http://127.0.0.1:9/v1",
        "",
        "local",
        None,
    );
    let cloud_endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        "http://api.cloud.test/v1",
        "k",
        "m2",
        None,
    );
    let permit =
        anybot_models::SendPermit::for_endpoint(&DataPolicy::LocalOnly, &local, ModelUse::Bot)
            .expect("permit");
    let err = gateway
        .complete(&cloud_endpoint, &empty_hi(), false, &permit)
        .await
        .err()
        .ok_or("cloud send accepted")?;
    let anybot_models::GatewayFail::Classified { classified, .. } = err else {
        return Err("expected classified".to_owned());
    };
    assert_eq!(classified.kind, anybot_models::FailKind::PolicyBlocked);
    assert!(cloud.recorded().is_empty());
    Ok(())
}

fn empty_hi() -> anybot_models::TurnRequest {
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
