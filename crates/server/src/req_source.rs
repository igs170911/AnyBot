use axum::http::HeaderMap;
use axum::http::header::HeaderName;

use anybot_domain::source_addr::source_address;

use crate::app::AppState;

pub const PROXY_SECRET_HEADER: HeaderName = HeaderName::from_static("x-anybot-proxy-secret");

pub const FORWARDED_HEADER: HeaderName = HeaderName::from_static("x-forwarded-for");

pub async fn request_source_address(headers: &HeaderMap, peer: &str, state: &AppState) -> String {
    let forwarded = header_text(headers, FORWARDED_HEADER);
    let config = state.config_view().await;
    let secret_ok = {
        let full = state.config.lock().await;
        proxy_secret_ok(headers, full.proxy_secret.as_ref())
    };
    let trust = config.trust_proxy.map(|ip| ip.to_string());
    let secret_configured = config.proxy_secret_configured;
    source_address(
        peer,
        trust.as_deref(),
        secret_ok,
        secret_configured,
        forwarded.as_deref(),
    )
}

fn header_text(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    let joined = headers
        .get_all(name)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect::<Vec<_>>()
        .join(",");
    (!joined.is_empty()).then_some(joined)
}

fn proxy_secret_ok(headers: &HeaderMap, expected: Option<&anybot_foundation::Secret>) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    headers
        .get(PROXY_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|given| constant_time_eq(given, expected.expose()))
}

fn constant_time_eq(given: &str, expected: &str) -> bool {
    let left = given.as_bytes();
    let right = expected.as_bytes();
    let longest = left.len().max(right.len());
    let mut left_iter = left.iter();
    let mut right_iter = right.iter();
    let mut difference: u8 = 0;
    for _ in 0..longest {
        let a = left_iter.next().copied().unwrap_or(0);
        let b = right_iter.next().copied().unwrap_or(0);
        difference |= a ^ b;
    }
    difference == 0 && left.len() == right.len()
}
