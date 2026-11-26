use anybot_foundation as _;
use anybot_models::{SwitchReason, ThinkingMode};
use bytes as _;
use futures_util as _;
use proptest::prelude::*;
use reqwest as _;
use rustls as _;
use serde as _;
use serde_json as _;
use test_harness::{ModelFormat, model_replies_text};
use tokio as _;
use tokio_rustls as _;
use trybuild as _;
use url as _;

#[path = "../test_support/session.rs"]
mod support;

#[tokio::test]
async fn outside_chain_becomes_primary() -> Result<(), String> {
    let (world, pair) = support::pair(ModelFormat::OpenAi).await?;
    support::script(&world.model, vec![model_replies_text("old", false)]);
    support::script(
        &pair.other,
        vec![
            model_replies_text("new", false),
            model_replies_text("again", false),
        ],
    );
    let mut session = support::session(&world)?;
    session.request_switch(&pair.fallback, SwitchReason::User);
    let first = support::ask_watching(&mut session, &pair.primary, &[], ThinkingMode::Auto).await?;
    assert_eq!(first.result.model, "m2");
    let second =
        support::ask_watching(&mut session, &pair.primary, &[], ThinkingMode::Auto).await?;
    assert_eq!(second.result.model, "m2");
    assert!(world.model.recorded().is_empty());
    assert_eq!(pair.other.recorded().len(), 2);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn handoff_records_cover_user_bot_fallback_unreachable(
        (reason_id, kind) in (0_u8..4, "[a-z-]{1,12}")
    ) {
        let reason = match reason_id {
            0 => SwitchReason::User,
            1 => SwitchReason::Bot,
            2 => SwitchReason::Unreachable,
            _ => SwitchReason::Fallback,
        };
        let kind_code = if matches!(reason, SwitchReason::Fallback) {
            Some(kind.as_str())
        } else {
            None
        };
        let (code, params) = anybot_models::handoff::reason_parts(reason, kind_code);
        prop_assert!(!code.is_empty());
        prop_assert_eq!(code, anybot_models::handoff_code(reason));
        if matches!(reason, SwitchReason::Fallback) {
            prop_assert!(params.iter().any(|(key, value)| key == "kind" && value == &kind));
        }
    }
}
