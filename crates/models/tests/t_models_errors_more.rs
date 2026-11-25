use trybuild as _;
#[path = "../test_support/permit.rs"]
mod permit_support;
use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use proptest::prelude::*;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{ErrorBodyKind, ModelFormat, model_error_body};
use tokio as _;
use url as _;

use anybot_models::{
    Audience, FailInput, FailKind, ProviderKind, RetryPlan, ThinkingMode, classify, try_limit,
};

#[path = "../test_support/mod.rs"]
mod support;

fn too_long_message() -> &'static str {
    "prompt is too long: 50000 maximum"
}

#[tokio::test]
async fn t_models_errors_2() -> Result<(), String> {
    let cases = [
        (
            ModelFormat::OpenAi,
            ProviderKind::OpenAi,
            ErrorBodyKind::OpenAi,
        ),
        (ModelFormat::Vllm, ProviderKind::Custom, ErrorBodyKind::Vllm),
        (
            ModelFormat::Anthropic,
            ProviderKind::Anthropic,
            ErrorBodyKind::Anthropic,
        ),
    ];
    for (format, kind, body) in cases {
        let world = support::world(format).await?;
        support::script(
            &world.model,
            vec![model_error_body(400, too_long_message(), body)],
        );
        let model = if matches!(kind, ProviderKind::Anthropic) {
            "claude-opus-5-5"
        } else {
            "m"
        };
        let endpoint = support::endpoint(kind, world.port, model, "k");
        let Err(err) = world
            .gateway
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                false,
                &permit_support::permit_for(&endpoint)?,
            )
            .await
        else {
            return Err("too long still succeeded".to_owned());
        };
        let anybot_models::GatewayFail::Classified { classified, .. } = err else {
            return Err("expected classified".to_owned());
        };
        assert_eq!(classified.kind, FailKind::TooLong);
        assert_eq!(classified.limit, Some(50_000));
        assert_eq!(world.model.recorded().len(), 1);
    }
    Ok(())
}

#[tokio::test]
async fn t_models_errors_3() -> Result<(), String> {
    let secret = "sk-live-do-not-log";
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![model_error_body(
            401,
            &format!("bad key {secret}"),
            ErrorBodyKind::OpenAi,
        )],
    );
    let endpoint = support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", secret);
    let mut session = anybot_models::Session::new(
        world.gateway.clone_gateway(),
        {
            let shared: std::sync::Arc<test_harness::FakeClock> =
                std::sync::Arc::clone(&world.clock);
            shared
        },
        0,
    );
    let primary = anybot_models::ModelRef {
        endpoint,
        bot_context: None,
    };
    let request = support::hi(ThinkingMode::Auto);
    let policy = anybot_models::DataPolicy::LocalOnly;
    let mut plan = anybot_models::ask_plan(&primary, &[], &policy, &request, Audience::Watching);
    let Err(err) = session.ask(&mut plan).await else {
        return Err("auth still succeeded".to_owned());
    };
    let shown = format!("{err:?}");
    assert!(!shown.contains(secret), "{shown}");
    for log in session.failure_logs() {
        assert!(!log.detail.contains(secret), "{}", log.detail);
    }
    Ok(())
}

fn input_strategy() -> impl Strategy<Value = FailInput> {
    prop_oneof![
        Just(FailInput::MissingKey),
        Just(FailInput::PolicyBlocked),
        Just(FailInput::Unreachable),
        Just(FailInput::FirstToken),
        Just(FailInput::Stalled),
        Just(FailInput::StreamBusy),
        Just(FailInput::StreamOther),
        Just(FailInput::RejectedParam),
        Just(FailInput::Redirect),
        Just(FailInput::Budget),
        (400_u16..500, any::<bool>()).prop_map(|(status, long)| FailInput::Http {
            status,
            message: if long {
                "prompt is too long: 50000 maximum".to_owned()
            } else {
                "no".to_owned()
            },
        }),
        (500_u16..530).prop_map(|status| FailInput::Http {
            status,
            message: "down".to_owned(),
        }),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn no_retry_kinds_plan_one_send((input, watching) in (input_strategy(), any::<bool>())) {
        let audience = if watching { Audience::Watching } else { Audience::Unattended };
        let row = classify(&input);
        let sends = if matches!(row.retry, RetryPlan::None) { 1 } else { try_limit(audience) };
        if matches!(row.retry, RetryPlan::None) {
            prop_assert_eq!(sends, 1);
        }
        prop_assert!(sends <= try_limit(audience));
    }
}
