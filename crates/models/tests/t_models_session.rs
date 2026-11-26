use proptest as _;
use std::time::Duration;
use trybuild as _;

use anybot_foundation as _;
use anybot_models::{Audience, FailKind, ProviderKind, ThinkingMode, ask_plan, gateway};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    ModelFormat, model_error_body, model_replies_text, model_reply_status_retry, timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/session_bound.rs"]
mod support;

#[tokio::test]
async fn too_long_does_not_switch_and_shrinks_limit() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_error_body(
                400,
                "maximum context length is 50000 tokens",
                test_harness::ErrorBodyKind::OpenAi,
            ),
            model_error_body(
                400,
                "maximum context length is 60000 tokens",
                test_harness::ErrorBodyKind::OpenAi,
            ),
        ],
    );
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k"),
        bot_context: None,
    };
    let mut session = support::session(&world)?;
    let first = session
        .ask(&mut ask_plan(
            &primary,
            &[],
            &support::hi(ThinkingMode::Auto),
            Audience::Watching,
        ))
        .await;
    assert!(
        matches!(first, Err(anybot_models::GatewayFail::Classified { classified, .. }) if classified.kind == FailKind::TooLong)
    );
    let again = session
        .ask(&mut ask_plan(
            &primary,
            &[],
            &support::hi(ThinkingMode::Auto),
            Audience::Watching,
        ))
        .await;
    assert!(again.is_err());
    let learned = session.learned().first().ok_or("missing learned")?;
    assert_eq!(learned.context_tokens, Some(50_000));
    let snapshot = session.learned().to_vec();
    let mut reopened = support::session(&world)?;
    reopened.load_learned(snapshot);
    let reopened_limit = reopened
        .learned()
        .first()
        .and_then(|row| row.context_tokens);
    assert_eq!(reopened_limit, Some(50_000));
    Ok(())
}

#[tokio::test]
async fn retry_after_is_capped_at_thirty_seconds() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status_retry(429, "slow", 50),
            model_replies_text("ok", false),
        ],
    );
    let clock = std::sync::Arc::clone(&world.clock);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let task = tokio::spawn(async move {
        let trusted = anybot_foundation::TrustedCerts::empty();
        let mut session = anybot_models::Session::new(
            gateway(support::share(&clock), &trusted)?,
            anybot_models::DataPolicy::LocalOnly,
            support::share(&clock),
            0,
        );
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        support::ask_watching(&mut session, &primary, &[], ThinkingMode::Auto).await
    });
    let ready = timeout_real(
        drive::drive_until(&world.clock, || task.is_finished()),
        Duration::from_secs(8),
    )
    .await;
    assert!(ready.is_ok());
    let outcome = task.await.map_err(|err| format!("{err}"))??;
    assert!(
        outcome
            .notices
            .iter()
            .any(|notice| notice.code == "model-retry-wait")
    );
    assert_eq!(world.model.recorded().len(), 2);
    assert!(
        world.clock.now_ms() >= 30_000,
        "retry waited {} ms",
        world.clock.now_ms()
    );
    Ok(())
}
