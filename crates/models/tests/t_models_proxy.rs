use rustls as _;
use std::sync::Arc;
use tokio_rustls as _;

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

#[tokio::test]
async fn loopback_neighbor_skips_the_configured_proxy() -> Result<(), String> {
    let (decoy, decoy_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (direct, direct_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    direct.script_set(vec![model_replies_text("ok", false)]);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let mut gateway = anybot_models::gateway_with_proxy(
        clock,
        &TrustedCerts::empty(),
        Some(&format!("http://127.0.0.1:{decoy_port}")),
    )?;
    gateway.pin_host(
        "localhost",
        std::net::SocketAddr::from(([127, 0, 0, 1], direct_port)),
    );
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://localhost:{direct_port}/v1"),
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
    Ok(())
}

#[tokio::test]
async fn classified_local_and_lan_skip_the_configured_proxy() -> Result<(), String> {
    let (decoy, decoy_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    let (direct, direct_port) = model_start_with_format(ModelFormat::OpenAi).await?;
    direct.script_set(vec![
        model_replies_text("mapped", false),
        model_replies_text("lan", false),
        model_replies_text("bare", false),
    ]);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let mut gateway = anybot_models::gateway_with_proxy(
        clock,
        &TrustedCerts::empty(),
        Some(&format!("http://127.0.0.1:{decoy_port}")),
    )?;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], direct_port));
    send_pinned(&mut gateway, "[::ffff:127.0.0.1]", addr, "mapped").await?;
    send_pinned(&mut gateway, "192.168.1.20", addr, "lan").await?;
    send_pinned(&mut gateway, "printer", addr, "bare").await?;
    assert_eq!(direct.recorded().len(), 3);
    assert!(decoy.recorded().is_empty());
    Ok(())
}

async fn send_pinned(
    gateway: &mut anybot_models::Gateway,
    host: &str,
    addr: std::net::SocketAddr,
    text: &str,
) -> Result<(), String> {
    gateway.pin_host(host.trim_matches(['[', ']']), addr);
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://{host}:{}/v1", addr.port()),
        "",
        "local",
        None,
    );
    let permit = permit_support::permit_for(&endpoint)?;
    let sent = gateway
        .complete(&endpoint, &hi(), false, &permit)
        .await
        .map_err(|err| format!("{host}: {err:?}"))?;
    assert_eq!(sent.text, text);
    Ok(())
}

#[test]
fn gateway_source_does_not_read_proxy_environment() {
    let sources = [
        include_str!("../src/client.rs"),
        include_str!("../src/client_http.rs"),
        include_str!("../src/client_place.rs"),
        include_str!("../src/reqwest_wire.rs"),
        include_str!("../src/catalog_fetch.rs"),
        include_str!("../src/lib.rs"),
    ];
    for source in sources {
        assert!(!source.contains("HTTP_PROXY"));
        assert!(!source.contains("HTTPS_PROXY"));
        assert!(!source.contains("ALL_PROXY"));
        let env_read = format!("{}::{}", "std::env", "var");
        assert!(!source.contains(&env_read));
    }
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
