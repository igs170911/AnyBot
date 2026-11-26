use proptest as _;
use rustls as _;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls as _;
use trybuild as _;

use anybot_foundation::clock::Clock;
use anybot_models::{Audience, DataPolicy, ProviderKind, Session, ThinkingMode, ask_plan, gateway};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    FakeClock, ModelFormat, model_replies_text, model_reply_status, sleep_real, timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/mod.rs"]
mod support;

fn share(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

#[tokio::test]
async fn t_models_n1_1() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
            model_replies_text("ok", false),
        ],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let _held = world.gateway;
    let clock = Arc::clone(&world.clock);
    let task = tokio::spawn(async move {
        let Ok(built) = gateway(share(&clock), &anybot_foundation::TrustedCerts::empty()) else {
            return Err(anybot_models::GatewayFail::MissingKey);
        };
        let mut session = Session::new(built, DataPolicy::LocalOnly, share(&clock), 0);
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        let request = support::hi(ThinkingMode::Auto);
        let mut plan = ask_plan(&primary, &[], &request, Audience::Watching);
        session.ask(&mut plan).await
    });
    let ready = timeout_real(drive(&world.clock, &task), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    assert_eq!(world.model.recorded().len(), 3);
    let outcome = task
        .await
        .map_err(|err| format!("{err}"))?
        .map_err(|err| format!("{err:?}"))?;
    assert_eq!(
        notice_values(&outcome, "seconds"),
        vec!["1".to_owned(), "2".to_owned()]
    );
    assert_eq!(
        notice_values(&outcome, "attempt")
            .first()
            .map(String::as_str),
        Some("1")
    );
    Ok(())
}

fn notice_values(outcome: &anybot_models::AskOutcome, name: &str) -> Vec<String> {
    outcome
        .notices
        .iter()
        .filter(|notice| notice.code == "model-retry-wait")
        .filter_map(|notice| {
            notice
                .params
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        })
        .collect()
}

#[tokio::test]
async fn t_models_n1_4() -> Result<(), String> {
    for status in [401_u16, 404] {
        let world = support::world(ModelFormat::OpenAi).await?;
        support::script(&world.model, vec![model_reply_status(status, "no")]);
        let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
        let mut session = Session::new(
            gateway(
                share(&world.clock),
                &anybot_foundation::TrustedCerts::empty(),
            )?,
            DataPolicy::LocalOnly,
            share(&world.clock),
            0,
        );
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        let request = support::hi(ThinkingMode::Auto);
        let mut plan = ask_plan(&primary, &[], &request, Audience::Watching);
        let _outcome = session.ask(&mut plan).await;
        assert_eq!(world.model.recorded().len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn unattended_sends_eight_times() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut steps = Vec::new();
    for _ in 0..8 {
        steps.push(model_reply_status(503, "down"));
    }
    support::script(&world.model, steps);
    let clock = Arc::clone(&world.clock);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let task = tokio::spawn(async move {
        let Ok(built) = gateway(share(&clock), &anybot_foundation::TrustedCerts::empty()) else {
            return Err(anybot_models::GatewayFail::MissingKey);
        };
        let mut session = Session::new(built, DataPolicy::LocalOnly, share(&clock), 200);
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        let request = support::hi(ThinkingMode::Auto);
        let mut plan = ask_plan(&primary, &[], &request, Audience::Unattended);
        session.ask(&mut plan).await
    });
    let ready = timeout_real(drive(&world.clock, &task), Duration::from_secs(8)).await;
    assert!(ready.is_ok());
    let joined = task.await.map_err(|err| format!("{err}"))?;
    assert!(joined.is_err());
    assert_eq!(world.model.recorded().len(), 8);
    Ok(())
}

#[test]
fn unattended_waits_stay_under_cap() {
    for attempt in 0..8 {
        let wait = anybot_models::base_wait(Audience::Unattended, attempt);
        assert!(wait.as_secs() <= 36);
    }
    assert_eq!(anybot_models::try_limit(Audience::Unattended), 8);
}

async fn drive(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::AskOutcome, anybot_models::GatewayFail>>,
) {
    loop {
        sleep_real(Duration::from_millis(20)).await;
        let _woke = clock.advance(Duration::from_secs(2));
        if task.is_finished() {
            break;
        }
    }
}
