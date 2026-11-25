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

#[cfg(test)]
mod tests {
    use super::*;

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
