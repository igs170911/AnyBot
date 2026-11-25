use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPlace {
    Local,
    Lan,
    Cloud,
}

#[must_use]
pub fn classify_url(raw: &str) -> RunPlace {
    let Ok(url) = Url::parse(raw) else {
        return RunPlace::Cloud;
    };
    match url.host() {
        Some(Host::Ipv4(addr)) => classify_v4(addr),
        Some(Host::Ipv6(addr)) => classify_v6(addr),
        Some(Host::Domain(name)) => classify_name(&name.to_ascii_lowercase()),
        None => RunPlace::Cloud,
    }
}

fn classify_v6(addr: Ipv6Addr) -> RunPlace {
    match addr.to_ipv4_mapped() {
        Some(v4) => classify_v4(v4),
        None => classify_addr(IpAddr::V6(addr)),
    }
}

fn classify_addr(addr: IpAddr) -> RunPlace {
    if addr.is_loopback() {
        return RunPlace::Local;
    }
    if lan_addr(addr) {
        return RunPlace::Lan;
    }
    RunPlace::Cloud
}

fn classify_v4(addr: Ipv4Addr) -> RunPlace {
    classify_addr(IpAddr::V4(addr))
}

fn classify_name(host: &str) -> RunPlace {
    if host == "localhost" {
        return RunPlace::Local;
    }
    if lan_name(host) {
        return RunPlace::Lan;
    }
    RunPlace::Cloud
}

fn lan_name(host: &str) -> bool {
    const SUFFIXES: [&str; 6] = [
        ".local",
        ".lan",
        ".home",
        ".internal",
        ".home.arpa",
        ".ts.net",
    ];
    if host.ends_with('.') {
        return false;
    }
    SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) || !host.contains('.')
}

fn lan_addr(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || shared_v4(v4),
        IpAddr::V6(v6) => unique_local(v6) || v6.is_unicast_link_local(),
    }
}

fn unique_local(addr: Ipv6Addr) -> bool {
    let head = addr.segments().first().copied().unwrap_or(0);
    (head & 0xfe00) == 0xfc00
}

fn shared_v4(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();
    octets.first() == Some(&100)
        && octets
            .get(1)
            .is_some_and(|second| (64..128).contains(second))
}

#[must_use]
pub fn first_token_wait(estimated_tokens: u64) -> std::time::Duration {
    let extra = estimated_tokens.saturating_sub(30_000) / 1_000;
    let seconds = 120_u64.saturating_add(extra.saturating_mul(2)).min(600);
    std::time::Duration::from_secs(seconds)
}

pub const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
pub const STALL_AFTER_TEXT: std::time::Duration = std::time::Duration::from_secs(35);
