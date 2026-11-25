use anybot_foundation as _;
use anybot_foundation::clock::Clock;
use anybot_models::{
    Audience, FailKind, ProviderKind, ThinkingMode, UsageReport, ask_plan, model_key,
};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use std::sync::Arc;
use test_harness::{
    FakeClock, ModelFormat, UsageScript, model_error_body, model_replies_text,
    model_start_with_format, model_usage,
};
use tokio as _;
use trybuild as _;
use url as _;

#[path = "../test_support/session.rs"]
mod support;

#[tokio::test]
async fn t_models_capabilities_3() -> Result<(), String> {
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
    let first = ask_once(&mut session, &primary).await;
    assert!(matches!(
        first,
        Err(anybot_models::GatewayFail::Classified { classified, .. })
            if classified.kind == FailKind::TooLong
    ));
    let again = ask_once(&mut session, &primary).await;
    assert!(again.is_err());
    assert_eq!(
        session.learned().first().and_then(|row| row.context_tokens),
        Some(50_000)
    );
    let snapshot = session.learned().to_vec();
    let mut reopened = anybot_models::Session::new(
        world_gateway(&world)?,
        anybot_models::DataPolicy::LocalOnly,
        support::share(&world.clock),
        0,
    );
    reopened.load_learned(snapshot);
    assert_eq!(
        reopened
            .learned()
            .first()
            .and_then(|row| row.context_tokens),
        Some(50_000)
    );
    Ok(())
}

async fn ask_once(
    session: &mut anybot_models::Session,
    primary: &anybot_models::ModelRef,
) -> Result<anybot_models::AskOutcome, anybot_models::GatewayFail> {
    session
        .ask(&mut ask_plan(
            primary,
            &[],
            &support::hi(ThinkingMode::Auto),
            Audience::Watching,
        ))
        .await
}

async fn ask_m1_again(
    session: &mut anybot_models::Session,
    primary: &anybot_models::ModelRef,
) -> Result<anybot_models::AskOutcome, String> {
    support::ask_watching(session, primary, &[], ThinkingMode::Deep).await
}

fn recorded_body(world: &support::World, index: usize) -> Result<String, String> {
    world
        .model
        .recorded()
        .get(index)
        .ok_or_else(|| "missing later ask".to_owned())
        .map(|row| row.body.clone())
}

fn world_gateway(world: &support::World) -> Result<anybot_models::Gateway, String> {
    anybot_models::gateway(
        support::share(&world.clock),
        &anybot_foundation::TrustedCerts::empty(),
    )
}

#[tokio::test]
async fn t_models_capabilities_4() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_error_body(
                400,
                "unsupported parameter: reasoning_effort",
                test_harness::ErrorBodyKind::OpenAi,
            ),
            model_replies_text("ok", false),
            model_replies_text("again", false),
        ],
    );
    support::script(&pair.other, vec![model_replies_text("m2", false)]);
    let mut session = support::session(&world)?;
    let first = support::ask_watching(&mut session, &pair.primary, &[], ThinkingMode::Deep).await?;
    assert_eq!(first.result.text, "ok");
    assert_eq!(world.model.recorded().len(), 2);
    let dropped = world
        .model
        .recorded()
        .get(1)
        .ok_or("missing retry")?
        .body
        .clone();
    assert!(!dropped.contains("reasoning_effort"));
    assert!(
        first
            .notices
            .iter()
            .all(|notice| notice.code != "model-retry-wait")
    );
    let again = ask_m1_again(&mut session, &pair.primary).await?;
    assert_eq!(again.result.text, "again");
    let later = recorded_body(&world, 2)?;
    assert!(!later.contains("reasoning_effort"));
    let second =
        support::ask_watching(&mut session, &pair.fallback, &[], ThinkingMode::Deep).await?;
    assert_eq!(second.result.text, "m2");
    let m2 = pair
        .other
        .recorded()
        .first()
        .ok_or("missing m2")?
        .body
        .clone();
    assert!(m2.contains("reasoning_effort"));
    Ok(())
}

#[tokio::test]
async fn t_models_capabilities_8() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    let mut steps = Vec::new();
    for _ in 0..6 {
        steps.push(model_usage(
            "ok",
            UsageScript {
                input_tokens: formula_of_hi().saturating_mul(13) / 10,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                output_tokens: 1,
                thinking_tokens: 0,
            },
            false,
        ));
    }
    support::script(&world.model, steps);
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::Custom, world.port, "local-model", ""),
        bot_context: Some(32_000),
    };
    let mut session = support::session(&world)?;
    let mut fifth = 0_u64;
    for index in 1..=6 {
        let outcome =
            support::ask_watching(&mut session, &primary, &[], ThinkingMode::Auto).await?;
        if index == 5 {
            fifth = match outcome.result.usage {
                UsageReport::Present(usage) => usage.input_total(),
                UsageReport::Absent => return Err("missing usage".to_owned()),
            };
        }
        if index == 6 {
            let handed = session.last_estimate().ok_or("missing estimate")?;
            assert!(handed >= fifth);
        }
    }
    let key = model_key(
        ProviderKind::Custom,
        &primary.endpoint.base_url,
        "local-model",
    );
    let learned = session
        .learned()
        .iter()
        .find(|row| row.key == key)
        .ok_or("missing calibration")?;
    let estimate = anybot_models::apply_calibration(formula_of_hi(), &learned.calibration);
    assert!(estimate >= fifth);
    Ok(())
}

fn formula_of_hi() -> u64 {
    anybot_models::formula_tokens(&anybot_models::EstimateInput {
        text: "hi",
        messages: 1,
        images: 0,
    })
}

#[tokio::test]
async fn t_models_capabilities_10() -> Result<(), String> {
    let (model, port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let clock = FakeClock::new(0);
    let shared: Arc<FakeClock> = Arc::clone(&clock);
    let clock_dyn: Arc<dyn Clock> = shared;
    let mut session = anybot_models::Session::new(
        anybot_models::gateway(clock_dyn, &anybot_foundation::TrustedCerts::empty())?,
        anybot_models::DataPolicy::LocalOnly,
        share(&clock),
        0,
    );
    let primary = anybot_models::ModelRef {
        endpoint: anybot_models::resolve_endpoint(
            ProviderKind::Custom,
            &format!("http://127.0.0.1:{port}/v1"),
            "",
            "local-model",
            None,
        ),
        bot_context: Some(10_000),
    };
    let mut request = support::hi(ThinkingMode::Auto);
    request.estimate_override = Some(11_000);
    let outcome = session
        .ask(&mut ask_plan(&primary, &[], &request, Audience::Watching))
        .await;
    let Err(err) = outcome else {
        return Err("budget still sent".to_owned());
    };
    let anybot_models::GatewayFail::Classified {
        classified,
        notices,
    } = err
    else {
        return Err("expected classified".to_owned());
    };
    assert_eq!(classified.code, "model-budget");
    assert_eq!(classified.kind, FailKind::Budget);
    assert!(
        notices.iter().any(|note| note.code == "model-budget") || classified.code == "model-budget"
    );
    assert!(model.recorded().is_empty());
    Ok(())
}

fn share(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}
