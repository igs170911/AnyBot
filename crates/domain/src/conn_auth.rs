use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginKey {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

#[must_use]
pub fn origin_keys(public_url: Option<&str>, listen: &[String], port: u16) -> Vec<OriginKey> {
    let mut keys: Vec<OriginKey> = public_url.and_then(parse_origin).into_iter().collect();

    for addr in listen {
        if is_wildcard(addr) {
            continue;
        }
        keys.push(OriginKey {
            scheme: "http".to_owned(),
            host: addr.to_owned(),
            port,
        });
    }
    if listen.iter().any(|addr| is_loopback(addr)) {
        keys.push(OriginKey {
            scheme: "http".to_owned(),
            host: "localhost".to_owned(),
            port,
        });
    }
    keys
}

fn is_wildcard(addr: &str) -> bool {
    addr == "0.0.0.0" || addr == "::"
}

fn is_loopback(addr: &str) -> bool {
    addr == "127.0.0.1" || addr == "::1"
}

#[must_use]
pub fn parse_origin(text: &str) -> Option<OriginKey> {
    let parsed = Url::parse(text).ok()?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    let host = parsed.host_str()?.to_ascii_lowercase();
    let port = parsed
        .port_or_known_default()
        .ok_or("origin port not resolvable")
        .ok()?;
    Some(OriginKey { scheme, host, port })
}

#[must_use]
pub fn origin_allowed(keys: &[OriginKey], origin: &str) -> bool {
    let Some(need) = parse_origin(origin) else {
        return false;
    };
    keys.iter().any(|key| key == &need)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrfDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsrfCreds {
    pub has_cookie: bool,
    pub has_bearer: bool,
    pub wants_cookie: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsrfFlags {
    pub write_method: bool,
    pub csrf_header_ok: bool,
    pub pre_bearer_endpoint: bool,
    pub creds: CsrfCreds,
}

#[must_use]
pub fn csrf_check(
    flags: &CsrfFlags,
    keys: &[OriginKey],
    origin: Option<&str>,
    sec_fetch: Option<&str>,
) -> CsrfDecision {
    if flags.creds.has_cookie && flags.creds.has_bearer {
        return CsrfDecision::Deny;
    }
    if !flags.write_method {
        if flags.creds.has_cookie && sec_fetch.is_some_and(|site| site != "same-origin") {
            return CsrfDecision::Deny;
        }
        return CsrfDecision::Allow;
    }
    match source_check(keys, origin, sec_fetch) {
        Some(checked) => checked,
        None => native_check(*flags),
    }
    .pipe_csrf(*flags)
}

fn source_check(
    keys: &[OriginKey],
    origin: Option<&str>,
    sec_fetch: Option<&str>,
) -> Option<CsrfDecision> {
    if let Some(origin) = origin {
        return Some(if origin_allowed(keys, origin) {
            CsrfDecision::Allow
        } else {
            CsrfDecision::Deny
        });
    }
    sec_fetch.map(|site| {
        if site == "same-origin" {
            CsrfDecision::Allow
        } else {
            CsrfDecision::Deny
        }
    })
}

fn native_check(flags: CsrfFlags) -> CsrfDecision {
    if flags.creds.has_cookie || flags.creds.wants_cookie {
        return CsrfDecision::Deny;
    }
    if flags.creds.has_bearer || flags.pre_bearer_endpoint {
        return CsrfDecision::Allow;
    }
    CsrfDecision::Deny
}

impl CsrfDecision {
    fn pipe_csrf(self, flags: CsrfFlags) -> CsrfDecision {
        if self == CsrfDecision::Allow && flags.creds.has_cookie && !flags.csrf_header_ok {
            return CsrfDecision::Deny;
        }
        self
    }
}
