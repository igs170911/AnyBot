use std::net::{IpAddr, Ipv4Addr};

pub const FORWARDED_MAX_LEN: usize = 64;

#[must_use]
pub fn last_forwarded_value(header: &str) -> Option<String> {
    let mut last = None;
    for part in header.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            last = Some(trimmed.to_owned());
        }
    }
    last.filter(|value| value.chars().count() <= FORWARDED_MAX_LEN)
}

#[must_use]
pub fn source_address(
    peer: &str,
    trust_proxy: Option<&str>,
    proxy_secret_ok: bool,
    secret_configured: bool,
    forwarded: Option<&str>,
) -> String {
    let trusting = trust_proxy.is_some_and(|proxy| proxy == peer);
    let secret_passes = !secret_configured || proxy_secret_ok;
    if trusting
        && secret_passes
        && let Some(value) = forwarded.and_then(last_forwarded_value)
    {
        return value;
    }
    peer.to_owned()
}

#[must_use]
pub fn peer_is_unlimited(peer: IpAddr) -> bool {
    match peer.to_canonical() {
        IpAddr::V4(v4) => v4_is_unlimited(v4),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

fn v4_is_unlimited(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || in_tailnet(ip)
}

fn in_tailnet(ip: Ipv4Addr) -> bool {
    let [first, second, _, _] = ip.octets();
    first == 100 && (second & 0xC0) == 64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlimited(text: &str) -> bool {
        peer_is_unlimited(text.parse().unwrap())
    }

    #[test]
    fn local_private_and_tailnet_peers_are_unlimited() {
        for peer in [
            "127.0.0.1",
            "::1",
            "10.1.2.3",
            "172.16.0.9",
            "192.168.1.20",
            "100.64.0.1",
            "100.127.255.254",
            "169.254.1.1",
            "fd7a:115c:a1e0::1",
            "fe80::1",
            "::ffff:192.168.1.20",
        ] {
            assert!(unlimited(peer), "{peer} must be unlimited");
        }
    }

    #[test]
    fn public_peers_are_limited() {
        for peer in [
            "1.2.3.4",
            "100.63.255.255",
            "100.128.0.1",
            "172.32.0.1",
            "2001:db8::1",
            "::ffff:1.2.3.4",
        ] {
            assert!(!unlimited(peer), "{peer} must be limited");
        }
    }

    #[test]
    fn forwarded_uses_last_value_and_length_cap() {
        assert_eq!(
            last_forwarded_value(" 9.9.9.9 , 1.2.3.4 ").as_deref(),
            Some("1.2.3.4"),
        );
        let long = "x".repeat(FORWARDED_MAX_LEN + 1);
        assert_eq!(last_forwarded_value(&long), None);
    }

    #[test]
    fn untrusted_peer_keeps_the_socket_address() {
        assert_eq!(
            source_address("10.0.0.8", Some("10.0.0.9"), true, false, Some("1.2.3.4")),
            "10.0.0.8",
        );
        assert_eq!(
            source_address("10.0.0.9", Some("10.0.0.9"), true, false, Some("1.2.3.4")),
            "1.2.3.4",
        );
        assert_eq!(
            source_address("10.0.0.9", Some("10.0.0.9"), false, true, Some("1.2.3.4")),
            "10.0.0.9",
        );
    }
}
