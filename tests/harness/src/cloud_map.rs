use std::collections::BTreeMap;
use std::net::SocketAddr;

#[derive(Clone, Debug, Default)]
pub struct NameMap {
    hosts: BTreeMap<String, SocketAddr>,
}

impl NameMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, host: &str, real: SocketAddr) {
        self.hosts.insert(host.to_owned(), real);
    }

    #[must_use]
    pub fn resolve(&self, requested: &str) -> Option<String> {
        let Ok(mut url) = url::Url::parse(requested) else {
            return None;
        };
        let host = url.host_str()?.to_owned();
        let real = self.hosts.get(&host)?;
        let Ok(()) = url.set_ip_host(real.ip()) else {
            return None;
        };
        let Ok(()) = url.set_port(Some(real.port())) else {
            return None;
        };
        Some(url.to_string())
    }
}

#[must_use]
pub fn cloud_name_matches(requested: &str, alias: &str) -> bool {
    let Ok(url) = url::Url::parse(requested) else {
        return false;
    };
    url.host_str() == Some(alias)
}

#[must_use]
pub fn mapped_origin(requested: &str, alias: &str, real: SocketAddr) -> Option<String> {
    if !cloud_name_matches(requested, alias) {
        return None;
    }
    let Ok(mut url) = url::Url::parse(requested) else {
        return None;
    };
    let Ok(()) = url.set_ip_host(real.ip()) else {
        return None;
    };
    let Ok(()) = url.set_port(Some(real.port())) else {
        return None;
    };
    Some(url.to_string())
}
