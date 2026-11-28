use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::pin::Pin;

use url::{Host, Url};

pub const ENDPOINT_MAX: usize = 2048;

pub type ResolveFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<IpAddr>, String>> + Send + 'a>>;

pub trait PushResolver: Send + Sync {
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl PushResolver for SystemResolver {
    fn resolve<'a>(&'a self, host: &'a str, port: u16) -> ResolveFuture<'a> {
        Box::pin(async move {
            let found = tokio::net::lookup_host((host, port))
                .await
                .map_err(|source| source.to_string())?;
            Ok(found.map(|address| address.ip()).collect())
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetVerdict {
    Allowed,
    NotHttps,
    LocalName,
    Unresolvable,
    NotPublic,
}

#[must_use]
pub fn endpoint_shape_ok(endpoint: &str) -> bool {
    if endpoint.chars().count() > ENDPOINT_MAX {
        return false;
    }
    Url::parse(endpoint).is_ok_and(|url| url.scheme() == "https" && url.host().is_some())
}

#[must_use]
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    let carrier_grade = a == 100 && (64..=127).contains(&b);
    let benchmarking = a == 198 && (18..=19).contains(&b);
    let protocol_assignments = a == 192 && b == 0 && c == 0;
    let reserved = a >= 240 || a == 0;
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified()
        || ip.is_multicast()
        || carrier_grade
        || benchmarking
        || protocol_assignments
        || reserved)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_v4(mapped);
    }
    let first = ip.segments().first().copied().unwrap_or(0);
    let unique_local = first & 0xfe00 == 0xfc00;
    let link_local = first & 0xffc0 == 0xfe80;
    let documentation = first == 0x2001 && ip.segments().get(1).copied() == Some(0x0db8);
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || unique_local
        || link_local
        || documentation)
}

fn is_local_name(domain: &str) -> bool {
    let lower = domain.trim_end_matches('.').to_ascii_lowercase();
    matches!(lower.rsplit('.').next(), Some("local" | "localhost"))
}

async fn check_domain(domain: &str, port: u16, resolver: &dyn PushResolver) -> TargetVerdict {
    if is_local_name(domain) {
        return TargetVerdict::LocalName;
    }
    let Ok(found) = resolver.resolve(domain, port).await else {
        return TargetVerdict::Unresolvable;
    };
    if found.is_empty() {
        return TargetVerdict::Unresolvable;
    }
    if found.into_iter().all(is_public_ip) {
        TargetVerdict::Allowed
    } else {
        TargetVerdict::NotPublic
    }
}

pub async fn check_target(endpoint: &str, resolver: &dyn PushResolver) -> TargetVerdict {
    let Ok(url) = Url::parse(endpoint) else {
        return TargetVerdict::NotHttps;
    };
    if url.scheme() != "https" {
        return TargetVerdict::NotHttps;
    }
    let port = url.port_or_known_default().unwrap_or(443);
    match url.host() {
        Some(Host::Domain(domain)) => check_domain(domain, port, resolver).await,
        Some(Host::Ipv4(ip)) => verdict_of_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => verdict_of_ip(IpAddr::V6(ip)),
        None => TargetVerdict::NotHttps,
    }
}

fn verdict_of_ip(ip: IpAddr) -> TargetVerdict {
    if is_public_ip(ip) {
        TargetVerdict::Allowed
    } else {
        TargetVerdict::NotPublic
    }
}
