use std::sync::Arc;

use anybot_foundation::clock::{Clock, SystemClock};
use anybot_models::{ProviderKind, StopKind, TurnResult, gateway, resolve_endpoint};

use crate::cases::{product_cases, product_cases_for};
use crate::profile::LiveProfile;
use crate::side::Outcome;
use test_harness::{ModelFormat, model_start_with_format};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeutralShape {
    pub stop: String,
    pub has_text: bool,
    pub has_thinking: bool,
    pub tool_calls: usize,
    pub usage: String,
}

pub async fn fake_product_shapes(
    port: u16,
    model: &str,
    always_thinks: bool,
) -> Outcome<Vec<NeutralShape>> {
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("http://127.0.0.1:{port}/v1"),
        "k",
        model,
        None,
    );
    complete_product_cases(
        endpoint,
        "fake",
        &anybot_foundation::TrustedCerts::empty(),
        always_thinks,
    )
    .await
}

pub async fn live_product_shapes(profile: &LiveProfile) -> Outcome<Vec<NeutralShape>> {
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &profile.base_url,
        profile.key.expose(),
        &profile.model,
        None,
    );
    let trusted = match anybot_foundation::TrustedCerts::parse_pem(&profile.ca_pem) {
        Ok(trusted) => trusted,
        Err(message) => return Outcome::partial(Vec::new(), message),
    };
    complete_product_cases(endpoint, "live", &trusted, profile.always_thinks).await
}

pub async fn https_product_shapes(
    port: u16,
    model: &str,
    host: &str,
    trusted: &anybot_foundation::TrustedCerts,
) -> Outcome<Vec<NeutralShape>> {
    let endpoint = resolve_endpoint(
        ProviderKind::Custom,
        &format!("https://{host}:{port}/v1"),
        "k",
        model,
        None,
    );
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let gateway = match gateway(clock, trusted) {
        Ok(mut gateway) => {
            gateway.pin_host(host, addr);
            gateway
        }
        Err(message) => return Outcome::partial(Vec::new(), message),
    };
    finish_cases(gateway, endpoint, "live", false).await
}

pub async fn complete_labeled(
    endpoint: anybot_models::ProviderEndpoint,
    side: &str,
) -> Outcome<Vec<NeutralShape>> {
    complete_product_cases(
        endpoint,
        side,
        &anybot_foundation::TrustedCerts::empty(),
        false,
    )
    .await
}

async fn complete_product_cases(
    endpoint: anybot_models::ProviderEndpoint,
    side: &str,
    trusted: &anybot_foundation::TrustedCerts,
    always_thinks: bool,
) -> Outcome<Vec<NeutralShape>> {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let gateway = match gateway(clock, trusted) {
        Ok(gateway) => gateway,
        Err(message) => return Outcome::partial(Vec::new(), message),
    };
    finish_cases(gateway, endpoint, side, always_thinks).await
}

async fn finish_cases(
    gateway: anybot_models::Gateway,
    endpoint: anybot_models::ProviderEndpoint,
    side: &str,
    always_thinks: bool,
) -> Outcome<Vec<NeutralShape>> {
    let mut shapes = Vec::new();
    let permit = match permit_for(&endpoint) {
        Ok(permit) => permit,
        Err(message) => return Outcome::partial(shapes, message),
    };
    for case in product_cases_for(always_thinks) {
        let request = case.request.clone();
        let result = gateway
            .complete(&endpoint, &request, case.stream, &permit)
            .await
            .map_err(|err| format!("{side} product {} failed: {err:?}", case.name));
        match result {
            Ok(turn) => shapes.push(shape_of(&turn)),
            Err(message) => return Outcome::partial(shapes, message),
        }
    }
    Outcome::ok(shapes)
}

fn permit_for(
    endpoint: &anybot_models::ProviderEndpoint,
) -> Result<anybot_models::SendPermit, String> {
    anybot_models::SendPermit::for_endpoint(
        &anybot_models::DataPolicy::Cloud {
            allowed: vec![endpoint.kind],
        },
        endpoint,
        anybot_models::ModelUse::Bot,
    )
    .ok_or_else(|| "permit".to_owned())
}

#[must_use]
pub fn product_lines(fake: &[NeutralShape], live: &[NeutralShape]) -> Vec<String> {
    let mut lines = vec!["product client:".to_owned()];
    for (index, case) in product_cases().into_iter().enumerate() {
        lines.push(product_pair(case.name, fake.get(index), live.get(index)));
    }
    lines
}

fn product_pair(name: &str, left: Option<&NeutralShape>, right: Option<&NeutralShape>) -> String {
    match (left, right) {
        (Some(left), Some(right)) => pair_line(name, left, right),
        (None, None) => format!("  {name}: missing fake and live"),
        (None, Some(_)) => format!("  {name}: missing fake"),
        (Some(_), None) => format!("  {name}: missing live"),
    }
}

fn pair_line(name: &str, left: &NeutralShape, right: &NeutralShape) -> String {
    if left == right {
        return format!("  {name}: identical structure");
    }
    format!("  {name}: fake {left:?} live {right:?}")
}

pub async fn compare_fakes() -> Result<(), String> {
    let shapes = shapes_from(ModelFormat::Vllm).await?;
    let expected = expected_shapes();
    if shapes != expected {
        return Err(format!("neutral shapes differ: {shapes:?} vs {expected:?}"));
    }
    Ok(())
}

fn expected_shapes() -> Vec<NeutralShape> {
    vec![
        NeutralShape {
            stop: "finished".to_owned(),
            has_text: true,
            has_thinking: true,
            tool_calls: 0,
            usage: "present".to_owned(),
        },
        NeutralShape {
            stop: "finished".to_owned(),
            has_text: true,
            has_thinking: true,
            tool_calls: 0,
            usage: "present".to_owned(),
        },
        NeutralShape {
            stop: "needs-tool".to_owned(),
            has_text: false,
            has_thinking: false,
            tool_calls: 1,
            usage: "present".to_owned(),
        },
        NeutralShape {
            stop: "finished".to_owned(),
            has_text: true,
            has_thinking: true,
            tool_calls: 0,
            usage: "present".to_owned(),
        },
    ]
}

async fn shapes_from(format: ModelFormat) -> Result<Vec<NeutralShape>, String> {
    let (model, port) = model_start_with_format(format).await?;
    model.script_set(crate::cases::product_steps_for(true));
    let shapes = fake_product_shapes(port, "local", true).await;
    drop(model);
    match shapes.failed {
        Some(message) => Err(message),
        None => Ok(shapes.side),
    }
}

fn shape_of(result: &TurnResult) -> NeutralShape {
    NeutralShape {
        stop: match result.stop {
            StopKind::Finished => "finished",
            StopKind::NeedsTool => "needs-tool",
            StopKind::Length => "length",
            StopKind::Refusal => "refusal",
            StopKind::Interrupted => "interrupted",
        }
        .to_owned(),
        has_text: !result.text.is_empty(),
        has_thinking: !result.thinking.is_empty(),
        tool_calls: result.tool_calls.len(),
        usage: match result.usage {
            anybot_models::UsageReport::Absent => "absent",
            anybot_models::UsageReport::Present(_) => "present",
        }
        .to_owned(),
    }
}

#[must_use]
pub fn missing_profile_hint() -> &'static str {
    "create .local/live-model/profile.toml with base_url, model, ca_file, key_file"
}
