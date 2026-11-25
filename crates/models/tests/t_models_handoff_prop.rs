use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness::{ModelFormat, model_replies_text};
use tokio as _;
use url as _;

use anybot_models::{SwitchReason, ThinkingMode};

#[path = "../test_support/session.rs"]
mod support;

#[tokio::test]
async fn switches_between_asks_keep_one_model() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    let mut steps = Vec::new();
    for _ in 0..1_000 {
        steps.push(model_replies_text("a", true));
    }
    support::script(&world.model, steps.clone());
    support::script(&pair.other, steps);
    let mut session = support::session(&world);
    let mut active = "m1";
    for index in 0..1_000 {
        let switching = index % 3 == 1 && active == "m1";
        if switching {
            session.request_switch(&pair.fallback, SwitchReason::User);
        }
        let primary = if active == "m1" {
            &pair.primary
        } else {
            &pair.fallback
        };
        let outcome = support::ask_watching(&mut session, primary, &[], ThinkingMode::Auto).await?;
        if switching {
            if outcome.result.model != "m2" {
                return Err(format!("switch missed at {index}"));
            }
            if outcome.handoffs.is_empty() {
                return Err("missing handoff record".to_owned());
            }
            active = "m2";
        } else if outcome.result.model != active {
            return Err(format!("model changed at {index}"));
        }
    }
    Ok(())
}
