use std::sync::Arc;

use bytes as _;
use futures_util as _;
use proptest::prelude::*;
use reqwest as _;
use serde as _;
use serde_json as _;
use trybuild as _;
use url as _;

use anybot_foundation::TrustedCerts;
use anybot_foundation::clock::{Clock, SystemClock};
use anybot_models::{ProviderKind, Role, ThinkingMode, TurnRequest, resolve_endpoint};
use test_harness::{
    IssuedLeaf, mint_issued, mint_issued_not_yet, mint_issued_window, mint_not_yet,
    mint_self_signed, model_replies_text, model_start_https, model_start_with_cert,
};

#[path = "../test_support/permit.rs"]
mod permit_support;

const HOST: &str = "model.invalid";

#[tokio::test]
async fn self_signed_ca_bit_requires_the_same_bytes() -> Result<(), String> {
    let started = model_start_https(HOST).await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let missing = ask(&TrustedCerts::empty(), HOST, started.port).await;
    assert_cert(&missing, "model-cert")?;
    assert!(started.model.recorded().is_empty());
    let listed = TrustedCerts::parse_pem(&started.cert.pem)?;
    let ok = ask(&listed, HOST, started.port).await?;
    assert_eq!(ok.text, "ok");
    let other = mint_self_signed(HOST, 2)?;
    let swapped = ask(&TrustedCerts::parse_pem(&other.pem)?, HOST, started.port).await;
    assert_cert(&swapped, "model-cert-untrusted")?;
    Ok(())
}

#[tokio::test]
async fn listed_issuer_connects() -> Result<(), String> {
    let issued = mint_issued(HOST, "ca.invalid")?;
    let started = model_start_with_cert(issued.cert).await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let trusted = TrustedCerts::parse_pem(&issued.issuer_pem)?;
    let ok = ask(&trusted, HOST, started.port).await?;
    assert_eq!(ok.text, "ok");
    Ok(())
}

#[tokio::test]
async fn name_mismatch_and_expiry_send_no_body() -> Result<(), String> {
    let named = mint_self_signed("a.invalid", 2)?;
    let started = model_start_with_cert(named).await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let trusted = TrustedCerts::parse_pem(&started.cert.pem)?;
    let mismatch = ask(&trusted, "b.invalid", started.port).await;
    assert_cert(&mismatch, "model-cert-name")?;
    assert!(started.model.recorded().is_empty());
    let expired = mint_self_signed(HOST, -1)?;
    let late = model_start_with_cert(expired).await?;
    late.model.script_set(vec![model_replies_text("ok", false)]);
    let trusted = TrustedCerts::parse_pem(&late.cert.pem)?;
    let old = ask(&trusted, HOST, late.port).await;
    assert_cert(&old, "model-cert-expired")?;
    assert!(late.model.recorded().is_empty());
    Ok(())
}

#[tokio::test]
async fn exact_and_issuer_paths_cover_four_faults() -> Result<(), String> {
    check_exact_faults().await?;
    check_issuer_faults().await
}

async fn check_exact_faults() -> Result<(), String> {
    let good = mint_self_signed(HOST, 2)?;
    let started = model_start_with_cert(good).await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let trusted = TrustedCerts::parse_pem(&started.cert.pem)?;
    ask(&trusted, HOST, started.port)
        .await
        .map_err(|err| format!("exact good: {err}"))?;
    let mismatch = ask(&trusted, "other.invalid", started.port).await;
    assert_cert(&mismatch, "model-cert-name")?;
    let expired = mint_self_signed(HOST, -1)?;
    let late = model_start_with_cert(expired).await?;
    let late_trust = TrustedCerts::parse_pem(&late.cert.pem)?;
    assert_cert(
        &ask(&late_trust, HOST, late.port).await,
        "model-cert-expired",
    )?;
    let early = mint_not_yet(HOST)?;
    let soon = model_start_with_cert(early).await?;
    let soon_trust = TrustedCerts::parse_pem(&soon.cert.pem)?;
    assert_cert(
        &ask(&soon_trust, HOST, soon.port).await,
        "model-cert-expired",
    )?;
    let stranger = mint_self_signed(HOST, 2)?;
    let other = model_start_with_cert(stranger).await?;
    assert_cert(
        &ask(&trusted, HOST, other.port).await,
        "model-cert-untrusted",
    )?;
    Ok(())
}

async fn check_issuer_faults() -> Result<(), String> {
    let issued = mint_issued(HOST, "ca.invalid")?;
    let started = model_start_with_cert(issued.cert).await?;
    started
        .model
        .script_set(vec![model_replies_text("ok", false)]);
    let trusted = TrustedCerts::parse_pem(&issued.issuer_pem)?;
    ask(&trusted, HOST, started.port)
        .await
        .map_err(|err| format!("issuer good: {err}"))?;
    assert_cert(
        &ask(&trusted, "other.invalid", started.port).await,
        "model-cert-name",
    )?;
    let expired = mint_issued_window(HOST, "ca.invalid", -1)?;
    let late = model_start_with_cert(expired.cert).await?;
    let late_trust = TrustedCerts::parse_pem(&expired.issuer_pem)?;
    assert_cert(
        &ask(&late_trust, HOST, late.port).await,
        "model-cert-expired",
    )?;
    let early = mint_issued_not_yet(HOST, "ca.invalid")?;
    let soon = model_start_with_cert(early.cert).await?;
    let soon_trust = TrustedCerts::parse_pem(&early.issuer_pem)?;
    assert_cert(
        &ask(&soon_trust, HOST, soon.port).await,
        "model-cert-expired",
    )?;
    let stranger = mint_issued(HOST, "other-ca.invalid")?;
    let other = model_start_with_cert(stranger.cert).await?;
    assert_cert(
        &ask(&trusted, HOST, other.port).await,
        "model-cert-untrusted",
    )?;
    Ok(())
}

#[tokio::test]
async fn pem_keeps_every_certificate_and_rejects_other_sections() -> Result<(), String> {
    let first = mint_self_signed("one.invalid", 2)?;
    let second = mint_self_signed("two.invalid", 2)?;
    let both = format!("{}{}", first.pem, second.pem);
    let parsed = TrustedCerts::parse_pem(&both)?;
    assert_eq!(parsed.ders().len(), 2);
    let junk = format!("{}\n{}\n", first.pem, non_certificate_section());
    assert!(TrustedCerts::parse_pem(&junk).is_err());
    Ok(())
}

fn non_certificate_section() -> String {
    let marker = ["BEG", "IN"].concat();
    let close = ["E", "ND"].concat();
    let kind = ["RSA", "PRIVATE", "KEY"].join(" ");
    format!("-----{marker} {kind}-----\nMIIB\n-----{close} {kind}-----")
}

async fn ask(
    trusted: &TrustedCerts,
    host: &str,
    port: u16,
) -> Result<anybot_models::TurnResult, String> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let built = anybot_models::gateway_pinned(clock, trusted, host, addr)?;
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("https://{host}:{port}/v1"),
        "k",
        "local",
        None,
    );
    let permit = permit_support::permit_for(&endpoint)?;
    built
        .complete(&endpoint, &hi(), false, &permit)
        .await
        .map_err(|err| format!("{err:?}"))
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

fn assert_cert(
    result: &Result<anybot_models::TurnResult, String>,
    code: &str,
) -> Result<(), String> {
    let Err(message) = result else {
        return Err("expected a certificate error".to_owned());
    };
    if message.contains(code) {
        return Ok(());
    }
    Err(message.clone())
}

fn _issued_type(issued: IssuedLeaf) -> String {
    issued.issuer_pem
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn untrusted_cert_never_passes(seed in 0u8..32, listed in 0usize..4) {
        let server = match mint_self_signed(HOST, 2) {
            Ok(server) => server,
            Err(message) => return Err(TestCaseError::fail(message)),
        };
        let mut pems = Vec::new();
        for index in 0..listed {
            let name = format!("other-{seed}-{index}.invalid");
            let extra = match mint_self_signed(&name, 2) {
                Ok(extra) => extra,
                Err(message) => return Err(TestCaseError::fail(message)),
            };
            pems.push(extra.pem);
        }
        let trusted = match TrustedCerts::parse_pem(&pems.concat()) {
            Ok(trusted) => trusted,
            Err(_) => TrustedCerts::empty(),
        };
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(err) => return Err(TestCaseError::fail(err.to_string())),
        };
        let failed = runtime.block_on(async {
            let started = match model_start_with_cert(server).await {
                Ok(started) => started,
                Err(message) => return Err(message),
            };
            ask(&trusted, HOST, started.port).await
        });
        prop_assert!(failed.is_err());
        let message = match failed {
            Err(message) => message,
            Ok(_) => String::new(),
        };
        prop_assert!(message.contains("model-cert-"));
    }
}
