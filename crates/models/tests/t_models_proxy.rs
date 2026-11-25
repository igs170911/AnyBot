use std::sync::Arc;

use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use tokio as _;
use trybuild as _;
use url as _;

use anybot_foundation::TrustedCerts;
use anybot_foundation::clock::{Clock, SystemClock};
use anybot_models::{ProviderKind, Role, ThinkingMode, TurnRequest, resolve_endpoint};
use test_harness::{ModelFormat, model_replies_text, model_start_with_format};

#[path = "../test_support/permit.rs"]
mod permit_support;

#[tokio::test]
async fn configured_proxy_is_used_and_localhost_skips_it() -> Result<(), String> {
    let (decoy, decoy_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (direct, direct_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    direct.script_set(vec![model_replies_text("ok", false)]);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let gateway = anybot_models::gateway_with_proxy(
        clock,
        &TrustedCerts::empty(),
        Some(&format!("http://127.0.0.1:{decoy_port}")),
    )?;
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{direct_port}/v1"),
        "",
        "local",
        None,
    );
    let permit = permit_support::permit_for(&endpoint)?;
    let sent = gateway
        .complete(&endpoint, &hi(), false, &permit)
        .await
        .map_err(|err| format!("{err:?}"))?;
    assert_eq!(sent.text, "ok");
    assert_eq!(direct.recorded().len(), 1);
    assert!(decoy.recorded().is_empty());
    assert!(gateway.os_notes().iter().all(|note| !note.is_empty()));
    Ok(())
}

fn hi() -> TurnRequest {
    TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking: ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}
