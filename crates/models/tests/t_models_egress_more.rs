use rustls as _;
use tokio_rustls as _;
use trybuild as _;
#[path = "../test_support/permit.rs"]
mod permit_support;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation as _;
use anybot_foundation::clock::Clock;
use anybot_models::{
    Audience, DataPolicy, FailKind, ModelUse, ProviderKind, ThinkingMode, allowed, ask_plan,
};
use bytes as _;
use futures_util as _;
use proptest::prelude::*;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    FakeClock, ModelFormat, model_reply_status, model_start_with_format, timeout_real,
};
use tiktoken_rs as _;
use tokio as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/wire.rs"]
mod support;

fn cloud_gateway(
    clock: &Arc<FakeClock>,
    cloud_port: u16,
) -> Result<anybot_models::Gateway, String> {
    let mut gateway =
        anybot_models::gateway(share(clock), &anybot_foundation::TrustedCerts::empty())?;
    gateway.pin_host(
        "api.cloud.test",
        std::net::SocketAddr::from(([127, 0, 0, 1], cloud_port)),
    );
    Ok(gateway)
}

fn share(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

async fn drive(clock: &FakeClock, task: &tokio::task::JoinHandle<bool>) {
    let _ready = timeout_real(
        drive::drive_until(clock, || task.is_finished()),
        Duration::from_secs(30),
    )
    .await;
}

#[tokio::test]
async fn t_models_egress_2() -> Result<(), String> {
    let (local, local_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (cloud, cloud_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    local.script_set(vec![
        model_reply_status(503, "down"),
        model_reply_status(503, "down"),
        model_reply_status(503, "down"),
    ]);
    let clock = FakeClock::new(0);
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::Custom, local_port, "m1", ""),
        bot_context: None,
    };
    let fallback = cloud_ref();
    let mut gateway = cloud_gateway(&clock, cloud_port)?;
    gateway.pin_host(
        "127.0.0.1",
        std::net::SocketAddr::from(([127, 0, 0, 1], local_port)),
    );
    let mut session = anybot_models::Session::new(gateway, DataPolicy::LocalOnly, share(&clock), 0);
    let request = support::hi(ThinkingMode::Auto);
    let clock_drive = Arc::clone(&clock);
    let task = tokio::spawn(async move {
        let err = session
            .ask(&mut ask_plan(
                &primary,
                std::slice::from_ref(&fallback),
                &request,
                Audience::Watching,
            ))
            .await;
        matches!(
            err,
            Err(anybot_models::GatewayFail::Classified { notices, .. })
                if notices.iter().any(|note| note.code == "model-policy-skip")
                    && notices.iter().any(|note| note.code == "model-server")
        )
    });
    drive(&clock_drive, &task).await;
    let skipped = task.await.map_err(|err| format!("{err}"))?;
    if !skipped {
        return Err(format!("local recorded {}", local.recorded().len()));
    }
    assert!(cloud.recorded().is_empty());
    assert_eq!(local.recorded().len(), 3);
    Ok(())
}

fn cloud_ref() -> anybot_models::ModelRef {
    anybot_models::ModelRef {
        endpoint: anybot_models::resolve_endpoint(
            ProviderKind::OpenAi,
            "http://api.cloud.test/v1",
            "k",
            "m2",
            None,
        ),
        bot_context: None,
    }
}

#[tokio::test]
async fn t_models_egress_3() -> Result<(), String> {
    let (local, local_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (cloud, cloud_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    local.script_set(vec![
        model_reply_status(503, "down"),
        model_reply_status(503, "down"),
        model_reply_status(503, "down"),
    ]);
    cloud.script_set(vec![test_harness::model_replies_text("ok", false)]);
    let clock = FakeClock::new(0);
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::Custom, local_port, "m1", ""),
        bot_context: None,
    };
    let fallback = anybot_models::ModelRef {
        endpoint: anybot_models::resolve_endpoint(
            ProviderKind::Custom,
            "http://api.cloud.test/v1",
            "",
            "m2",
            None,
        ),
        bot_context: None,
    };
    let mut gateway = cloud_gateway(&clock, cloud_port)?;
    gateway.pin_host(
        "127.0.0.1",
        std::net::SocketAddr::from(([127, 0, 0, 1], local_port)),
    );
    let mut session = anybot_models::Session::new(
        gateway,
        DataPolicy::Cloud {
            allowed: vec![ProviderKind::Custom],
        },
        share(&clock),
        0,
    );
    let request = support::hi(ThinkingMode::Auto);
    let clock_drive = Arc::clone(&clock);
    let task = tokio::spawn(async move {
        let outcome = session
            .ask(&mut ask_plan(
                &primary,
                std::slice::from_ref(&fallback),
                &request,
                Audience::Watching,
            ))
            .await;
        outcome.is_ok()
    });
    drive(&clock_drive, &task).await;
    let ok = task.await.map_err(|err| format!("{err}"))?;
    assert!(ok, "cloud recorded {}", cloud.recorded().len());
    assert_eq!(cloud.recorded().len(), 1);
    Ok(())
}

#[tokio::test]
async fn t_models_egress_4() -> Result<(), String> {
    let (local, local_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (cloud, cloud_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    local.script_set(vec![test_harness::model_redirect(
        "http://api.cloud.test/v1/chat/completions",
    )]);
    let clock = FakeClock::new(0);
    let gateway = cloud_gateway(&clock, cloud_port)?;
    let endpoint = support::endpoint(ProviderKind::Custom, local_port, "m1", "");
    let err = gateway
        .complete(
            &endpoint,
            &support::hi(ThinkingMode::Auto),
            false,
            &permit_support::permit_for(&endpoint)?,
        )
        .await
        .err()
        .ok_or("redirect followed")?;
    let anybot_models::GatewayFail::Classified { classified, .. } = err else {
        return Err("expected classified".to_owned());
    };
    assert_eq!(classified.kind, FailKind::Redirect);
    assert!(cloud.recorded().is_empty());
    assert!(!allowed(
        &DataPolicy::LocalOnly,
        "http://api.cloud.test/v1",
        ProviderKind::OpenAi,
        ModelUse::Bot,
    ));
    Ok(())
}

fn place_of(url: &str) -> bool {
    matches!(
        anybot_models::classify_url(url),
        anybot_models::RunPlace::Cloud
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn local_policy_never_plans_cloud(
        (local_only, use_id, cloud) in (any::<bool>(), 0_u8..4, any::<bool>())
    ) {
        let policy = if local_only {
            DataPolicy::LocalOnly
        } else {
            DataPolicy::Cloud { allowed: vec![ProviderKind::OpenAi] }
        };
        let use_kind = match use_id {
            0 => ModelUse::Summary,
            1 => ModelUse::Embed,
            2 => ModelUse::Rerank,
            _ => ModelUse::Bot,
        };
        let url = if cloud {
            "https://api.openai.com/v1"
        } else {
            "http://127.0.0.1:9/v1"
        };
        let sent = allowed(&policy, url, ProviderKind::OpenAi, use_kind);
        if local_only && place_of(url) {
            prop_assert!(!sent);
        }
        if matches!(use_kind, ModelUse::Summary | ModelUse::Embed | ModelUse::Rerank) && place_of(url)
        {
            prop_assert!(!sent);
        }
    }
}
