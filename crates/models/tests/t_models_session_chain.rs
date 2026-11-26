use rustls as _;
use std::time::Duration;
use tokio_rustls as _;

use anybot_foundation as _;
use anybot_models::ProviderKind;
use anybot_models::{ThinkingMode, gateway};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    ModelFormat, model_replies_text, model_reply_status, model_start_with_format, timeout_real,
};
use tokio as _;
use trybuild as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/stopped.rs"]
mod stop_slot;
#[path = "../test_support/session.rs"]
mod support;

fn keep_stopped() -> bool {
    let (tx, _rx) = stop_slot::stopped();
    tx.send(false).is_ok()
}

#[tokio::test]
async fn missing_key_fallback_is_skipped() -> Result<(), String> {
    let (world, mut pair) = support::pair(ModelFormat::OpenAi).await?;
    pair.fallback.endpoint = support::endpoint(ProviderKind::OpenAi, 1, "m2", "");
    let (third_model, third_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    third_model.script_set(vec![model_replies_text("m3", false)]);
    let third = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, third_port, "m3", "k"),
        bot_context: None,
    };
    support::script(&world.model, vec![model_reply_status(401, "no")]);
    let mut session = support::session(&world)?;
    assert_eq!(pair.fallback.endpoint.model, "m2");
    let _other = pair.other.recorded().len();
    let outcome = support::ask_watching(
        &mut session,
        &pair.primary,
        &[pair.fallback, third],
        ThinkingMode::Auto,
    )
    .await?;
    assert_eq!(outcome.result.text, "m3");
    assert!(
        outcome
            .skips
            .iter()
            .any(|notice| notice.code == "model-missing-key")
    );
    assert_eq!(third_model.recorded().len(), 1);
    Ok(())
}

#[tokio::test]
async fn fallback_after_retries_uses_second_model() -> Result<(), String> {
    let _kept = keep_stopped();
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
        ],
    );
    support::script(&pair.other, vec![model_replies_text("ok", false)]);
    let clock = std::sync::Arc::clone(&world.clock);
    let primary = pair.primary;
    let fallback = pair.fallback;
    let task = tokio::spawn(async move {
        let trusted = anybot_foundation::TrustedCerts::empty();
        let mut session = anybot_models::Session::new(
            gateway(support::share(&clock), &trusted)?,
            anybot_models::DataPolicy::LocalOnly,
            support::share(&clock),
            0,
        );
        support::ask_watching(
            &mut session,
            &primary,
            std::slice::from_ref(&fallback),
            ThinkingMode::Deep,
        )
        .await
    });
    let ready = timeout_real(
        drive::drive_until(&world.clock, || task.is_finished()),
        Duration::from_secs(8),
    )
    .await;
    assert!(ready.is_ok());
    let outcome = task.await.map_err(|err| format!("{err}"))??;
    assert_eq!(outcome.result.text, "ok");
    assert_eq!(world.model.recorded().len(), 3);
    assert_eq!(pair.other.recorded().len(), 1);
    let handoff = outcome.handoffs.first().ok_or("missing handoff")?;
    assert_eq!(handoff.before_model, "m1");
    assert_eq!(handoff.after_model, "m2");
    assert_eq!(handoff.reason, anybot_models::SwitchReason::Fallback);
    let fallback_body = pair
        .other
        .recorded()
        .first()
        .map(|row| row.body.clone())
        .ok_or("missing fallback body")?;
    assert!(!fallback_body.contains("reasoning_effort"));
    Ok(())
}

#[tokio::test]
async fn resting_model_is_skipped_until_clock_passes() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_reply_status(401, "no")]);
    support::script(
        &pair.other,
        vec![
            model_replies_text("first", false),
            model_replies_text("second", false),
            model_replies_text("later", false),
        ],
    );
    let mut session = support::session(&world)?;
    let first = support::ask_watching(
        &mut session,
        &pair.primary,
        std::slice::from_ref(&pair.fallback),
        ThinkingMode::Auto,
    )
    .await?;
    assert_eq!(first.result.text, "first");
    assert_eq!(world.model.recorded().len(), 1);
    let again = support::ask_watching(
        &mut session,
        &pair.primary,
        std::slice::from_ref(&pair.fallback),
        ThinkingMode::Auto,
    )
    .await;
    let again_text = again.ok().map(|row| row.result.text);
    assert_eq!(again_text.as_deref(), Some("second"));
    let _moved = world.clock.advance(Duration::from_mins(11));
    support::script(&world.model, vec![model_replies_text("back", false)]);
    let third = support::ask_watching(
        &mut session,
        &pair.primary,
        std::slice::from_ref(&pair.fallback),
        ThinkingMode::Auto,
    )
    .await?;
    assert_eq!(third.result.model, "m1");
    Ok(())
}
