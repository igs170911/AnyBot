use url::Url;

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
    let Some(host) = url.host_str() else {
        return RunPlace::Cloud;
    };
    classify_host(host)
}

fn classify_host(host: &str) -> RunPlace {
    let lowered = host.to_ascii_lowercase();
    if lowered == "localhost" || lowered == "::1" || lowered.starts_with("127.") {
        return RunPlace::Local;
    }
    if lan_name(&lowered) || lan_ip(&lowered) {
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
    SUFFIXES.iter().any(|suffix| host.ends_with(suffix)) || !host.contains('.')
}

fn lan_ip(host: &str) -> bool {
    let Ok(addr) = host.parse::<std::net::IpAddr>() else {
        return false;
    };
    match addr {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || shared_v4(v4),
        std::net::IpAddr::V6(v6) => {
            let segments = v6.segments();
            let head = segments.first().copied().unwrap_or(0);
            (head & 0xfe00) == 0xfc00 || (head & 0xffc0) == 0xfe80
        }
    }
}

fn shared_v4(addr: std::net::Ipv4Addr) -> bool {
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
