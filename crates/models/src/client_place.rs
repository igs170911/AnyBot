use std::net::SocketAddr;

use crate::egress::SendPermit;
use crate::providers::key_allowed;
use crate::types::ProviderEndpoint;
use crate::where_run::{RunPlace, classify_url};

pub(crate) fn chat_url_of(endpoint: &ProviderEndpoint) -> String {
    crate::client_http::chat_url(endpoint)
}

pub(crate) fn key_for(endpoint: &ProviderEndpoint, listed: &str, dial: &str) -> bool {
    key_allowed(endpoint, listed) || key_allowed(endpoint, dial)
}

pub(crate) fn place_ok(
    permit: &SendPermit,
    listed: &str,
    dial: &str,
    provider: crate::types::ProviderKind,
) -> bool {
    let listed_place = classify_url(listed);
    let dial_place = classify_url(dial);
    if stricter(dial_place, listed_place) {
        return permit.allows_url(dial, provider);
    }
    true
}

fn stricter(candidate: RunPlace, baseline: RunPlace) -> bool {
    place_rank(candidate) > place_rank(baseline)
}

fn place_rank(place: RunPlace) -> u8 {
    match place {
        RunPlace::Local => 0,
        RunPlace::Lan => 1,
        RunPlace::Cloud => 2,
    }
}

pub(crate) fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
}

pub(crate) fn pin_url(url: &str, addr: SocketAddr) -> Option<String> {
    let mut parsed = url::Url::parse(url).ok()?;
    parsed.set_ip_host(addr.ip()).ok()?;
    if addr.port() != 0 {
        parsed.set_port(Some(addr.port())).ok()?;
    }
    Some(parsed.to_string())
}
