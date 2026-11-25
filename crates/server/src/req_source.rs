use axum::http::header::HeaderName;

use anybot_domain::source_addr::source_address;

use crate::app::AppState;

pub const PROXY_SECRET_HEADER: HeaderName = HeaderName::from_static("x-anybot-proxy-secret");

pub const FORWARDED_HEADER: HeaderName = HeaderName::from_static("x-forwarded-for");

pub fn request_source_address(
    request: &axum::extract::Request,
    peer: &str,
    state: &AppState,
) -> String {
    let forwarded = header_text(request.headers(), FORWARDED_HEADER);
    let secret_ok = proxy_secret_ok(request.headers(), state);
    let trust = state.config.trust_proxy.map(|ip| ip.to_string());
    let secret_configured = state.config.proxy_secret.is_some();
    source_address(
        peer,
        trust.as_deref(),
        secret_ok,
        secret_configured,
        forwarded.as_deref(),
    )
}

fn header_text(headers: &axum::http::HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn proxy_secret_ok(headers: &axum::http::HeaderMap, state: &AppState) -> bool {
    let Some(expected) = state.config.proxy_secret.as_ref() else {
        return true;
    };
    headers
        .get(PROXY_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|given| given == expected.expose())
}
