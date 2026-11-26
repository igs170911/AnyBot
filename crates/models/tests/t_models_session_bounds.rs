use anybot_foundation as _;
use anybot_models::{ProviderKind, ThinkingMode};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness::{ModelFormat, model_replies_text, model_reply_status};
use tokio as _;
use tokio_rustls as _;
use trybuild as _;
use url as _;

#[path = "../test_support/drive.rs"]
mod drive;
#[path = "../test_support/session_bound.rs"]
mod support;

#[tokio::test]
async fn rejected_thinking_is_dropped_on_the_next_send() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(400, "unsupported parameter: reasoning_effort"),
            model_replies_text("ok", false),
            model_replies_text("ok2", false),
        ],
    );
    let mut asked = ask_deep(&world).await?;
    assert_eq!(asked.text, "ok");
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k"),
        bot_context: None,
    };
    let second =
        support::ask_watching(asked.session_mut(), &primary, &[], ThinkingMode::Deep).await?;
    assert_eq!(second.result.text, "ok2");
    let bodies = bodies_of(&world);
    assert!(
        bodies
            .first()
            .ok_or("missing first")?
            .contains("reasoning_effort")
    );
    assert!(
        !bodies
            .get(1)
            .ok_or("missing second")?
            .contains("reasoning_effort")
    );
    assert!(
        !bodies
            .get(2)
            .ok_or("missing third")?
            .contains("reasoning_effort")
    );
    Ok(())
}

#[tokio::test]
async fn second_rejection_of_the_same_parameter_fails() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_reply_status(400, "unsupported parameter: reasoning_effort"),
            model_reply_status(400, "unsupported parameter: reasoning_effort"),
            model_replies_text("ok", false),
        ],
    );
    let failed = ask_deep(&world).await;
    assert!(failed.is_err());
    assert_eq!(world.model.recorded().len(), 2);
    Ok(())
}

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
    let asked = ask_auto(&world).await?;
    assert_eq!(asked, "ok");
    assert_eq!(world.model.recorded().len(), 2);
    Ok(())
}

#[tokio::test]
async fn second_blank_reply_is_a_failure() -> Result<(), String> {
    let world = support::world(ModelFormat::OpenAi).await?;
    support::script(
        &world.model,
        vec![
            model_replies_text("   ", false),
            model_replies_text("", false),
            model_replies_text("late", false),
        ],
    );
    let failed = ask_auto(&world).await;
    assert!(failed.is_err());
    assert_eq!(world.model.recorded().len(), 2);
    Ok(())
}

struct Asked {
    session: anybot_models::Session,
    text: String,
}

impl Asked {
    fn session_mut(&mut self) -> &mut anybot_models::Session {
        &mut self.session
    }
}

async fn ask_deep(world: &support::World) -> Result<Asked, String> {
    ask_with(world, ThinkingMode::Deep).await
}

async fn ask_auto(world: &support::World) -> Result<String, String> {
    Ok(ask_with(world, ThinkingMode::Auto).await?.text)
}

async fn ask_with(world: &support::World, thinking: ThinkingMode) -> Result<Asked, String> {
    let primary = anybot_models::ModelRef {
        endpoint: support::endpoint(ProviderKind::OpenAi, world.port, "gpt-5", "k"),
        bot_context: None,
    };
    let mut session = support::session(world)?;
    let clock = std::sync::Arc::clone(&world.clock);
    let driver = tokio::spawn(async move {
        drive::drive_until(&clock, || false).await;
    });
    let outcome = support::ask_watching(&mut session, &primary, &[], thinking).await;
    driver.abort();
    let outcome = outcome?;
    Ok(Asked {
        session,
        text: outcome.result.text,
    })
}

fn bodies_of(world: &support::World) -> Vec<String> {
    world
        .model
        .recorded()
        .into_iter()
        .map(|row| row.body)
        .collect()
}
