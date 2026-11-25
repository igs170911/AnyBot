use std::time::Duration;
use trybuild as _;

use anybot_foundation as _;
use anybot_models::{
    Audience, DataPolicy, FailKind, Learned, ProviderKind, SwitchReason, ThinkingMode, ToolSpec,
    ask_plan, gateway,
};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{
    ModelFormat, model_replies_text, model_reply_status, model_stream_error, timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/stopped.rs"]
mod stop_slot;
#[path = "../test_support/session.rs"]
mod support;

#[tokio::test]
async fn blank_reply_is_asked_again() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_replies_text("   ", false),
            model_replies_text("ok", false),
        ],
    );
    let mut session = support::session(&world)?;
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k"),
        bot_context: None,
    };
    let outcome = support::ask_watching(&mut session, &primary, &[], ThinkingMode::Auto).await?;
    assert_eq!(outcome.result.text, "ok");
    assert_eq!(world.model.recorded().len(), 2);
    Ok(())
}

#[tokio::test]
async fn stream_busy_waits_then_retries() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_stream_error(true, "engine is busy"),
            model_replies_text("ok", true),
        ],
    );
    let clock = std::sync::Arc::clone(&world.clock);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let task = tokio::spawn(async move {
        let mut session = anybot_models::Session::new(
            gateway(support::share(&clock))?,
            support::share(&clock),
            0,
        );
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        let request = support::hi(ThinkingMode::Auto);
        let mut plan = ask_plan(
            &primary,
            &[],
            &DataPolicy::LocalOnly,
            &request,
            Audience::Watching,
        );
        plan.stream = true;
        session
            .ask(&mut plan)
            .await
            .map_err(|err| format!("{err:?}"))
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
            .any(|notice| notice.code == "model-busy-wait")
    );
    assert_eq!(world.model.recorded().len(), 2);
    Ok(())
}

#[tokio::test]
async fn stop_during_retry_ends_quietly() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
        ],
    );
    let (tx, rx) = stop_slot::stopped();
    let clock = std::sync::Arc::clone(&world.clock);
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k");
    let task = tokio::spawn(async move {
        let mut session = anybot_models::Session::new(
            gateway(support::share(&clock))?,
            support::share(&clock),
            0,
        )
        .with_stop(rx);
        let primary = anybot_models::ModelRef {
            endpoint,
            bot_context: None,
        };
        support::ask_watching(&mut session, &primary, &[], ThinkingMode::Auto).await
    });
    test_harness::sleep_real(Duration::from_millis(80)).await;
    let _sent = tx.send(true);
    let _moved = world.clock.advance(Duration::from_secs(2));
    let ready = timeout_real(task, Duration::from_secs(5)).await;
    let joined = ready
        .map_err(|err| format!("{err:?}"))?
        .map_err(|err| format!("{err}"))?;
    assert!(matches!(joined, Err(ref text) if text.contains("Stopped")));
    Ok(())
}

#[tokio::test]
async fn policy_skip_does_not_call_cloud_fallback() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
            model_reply_status(503, "down"),
        ],
    );
    let clock = std::sync::Arc::clone(&world.clock);
    let mut primary = pair.primary;
    primary.endpoint = support::endpoint(ProviderKind::Ollama, world.port, "m1", "");
    let mut fallback = pair.fallback;
    fallback.endpoint.base_url = "http://api.cloud.test/v1".to_owned();
    let task = tokio::spawn(async move {
        let mut session = anybot_models::Session::new(
            gateway(support::share(&clock))?,
            support::share(&clock),
            0,
        );
        support::ask_watching(
            &mut session,
            &primary,
            std::slice::from_ref(&fallback),
            ThinkingMode::Auto,
        )
        .await
    });
    let ready = timeout_real(
        drive::drive_until(&world.clock, || task.is_finished()),
        Duration::from_secs(8),
    )
    .await;
    assert!(ready.is_ok());
    let outcome = task.await.map_err(|err| format!("{err}"))?;
    let err = outcome.err().ok_or("policy skip still succeeded")?;
    let shown = format!("{err:?}");
    assert!(shown.contains("model-policy-skip") || shown.contains("Server"));
    assert!(pair.other.recorded().is_empty());
    Ok(())
}

#[tokio::test]
async fn switch_request_applies_to_the_next_ask() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_replies_text("one", false),
            model_replies_text("nope", false),
        ],
    );
    support::script(&pair.other, vec![model_replies_text("two", false)]);
    let mut session = support::session(&world)?;
    let first = support::ask_watching(
        &mut session,
        &pair.primary,
        std::slice::from_ref(&pair.fallback),
        ThinkingMode::Auto,
    )
    .await?;
    assert_eq!(first.result.text, "one");
    session.request_switch(&pair.fallback, SwitchReason::User);
    let second = support::ask_watching(
        &mut session,
        &pair.primary,
        std::slice::from_ref(&pair.fallback),
        ThinkingMode::Auto,
    )
    .await?;
    assert_eq!(second.result.model, "m2");
    assert!(second.handoffs.iter().any(|row| row.after_model == "m2"));
    Ok(())
}

#[tokio::test]
async fn unsupported_tools_are_not_sent() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k"),
        bot_context: None,
    };
    let mut session = support::session(&world)?;
    let key = anybot_models::model_key(ProviderKind::OpenAi, &primary.endpoint.base_url, "gpt-5");
    let mut learned = Learned::new(&key);
    learned.tools = Some(false);
    session.load_learned(vec![learned]);
    let mut request = support::hi(ThinkingMode::Auto);
    request.tools.push(ToolSpec {
        name: "read_file".to_owned(),
        description: "read".to_owned(),
        parameters: serde_json::json!({}),
    });
    let mut plan = ask_plan(
        &primary,
        &[],
        &DataPolicy::LocalOnly,
        &request,
        Audience::Watching,
    );
    let outcome = session.ask(&mut plan).await;
    assert!(matches!(
        outcome,
        Err(anybot_models::GatewayFail::Classified { classified, .. }) if classified.kind == FailKind::NoTools
    ));
    assert!(world.model.recorded().is_empty());
    Ok(())
}

#[test]
fn errors_property_never_retries_terminal_kinds() {
    let kinds = [
        anybot_models::FailInput::MissingKey,
        anybot_models::FailInput::Http {
            status: 401,
            message: "no".to_owned(),
        },
        anybot_models::FailInput::Http {
            status: 404,
            message: "missing".to_owned(),
        },
        anybot_models::FailInput::Stalled,
    ];
    for _ in 0..250 {
        for input in &kinds {
            let row = anybot_models::classify(input);
            assert_eq!(row.retry, anybot_models::RetryPlan::None);
        }
    }
    assert!(anybot_models::try_limit(Audience::Watching) <= 3);
    assert!(anybot_models::try_limit(Audience::Unattended) <= 8);
}
