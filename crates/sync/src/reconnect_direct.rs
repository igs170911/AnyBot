use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

#[must_use]
pub fn goes_direct(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(addr)) => direct_v4(addr),
        Some(Host::Ipv6(addr)) => direct_v6(addr),
        Some(Host::Domain(name)) => direct_name(&name.to_ascii_lowercase()),
        None => false,
    }
}

fn direct_v6(addr: Ipv6Addr) -> bool {
    match addr.to_ipv4_mapped() {
        Some(v4) => direct_v4(v4),
        None => direct_addr(IpAddr::V6(addr)),
    }
}

fn direct_v4(addr: Ipv4Addr) -> bool {
    direct_addr(IpAddr::V4(addr))
}

fn direct_addr(addr: IpAddr) -> bool {
    if addr.is_loopback() {
        return true;
    }
    match addr {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || shared_v4(v4),
        IpAddr::V6(v6) => unique_local(v6) || v6.is_unicast_link_local(),
    }
}

fn direct_name(host: &str) -> bool {
    const SUFFIXES: [&str; 6] = [
        ".local",
        ".lan",
        ".home",
        ".internal",
        ".home.arpa",
        ".ts.net",
    ];
    if host == "localhost" {
        return true;
    }
    if host.ends_with('.') {
        return false;
    }
    SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) || !host.contains('.')
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

#[cfg(test)]
#[path = "reconnect_direct_tests.rs"]
mod tests;
