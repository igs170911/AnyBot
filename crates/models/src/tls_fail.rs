use crate::transport::TransportFail;

pub(crate) fn map_reqwest(err: &reqwest::Error) -> TransportFail {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    let mut chain = String::new();
    while let Some(item) = current {
        if let Some(tls) = item.downcast_ref::<rustls::Error>() {
            return TransportFail::Certificate(anybot_foundation::classify_tls(tls));
        }
        if chain.len() < 400 {
            chain.push_str(&item.to_string());
            chain.push(' ');
        }
        current = std::error::Error::source(item);
    }
    if let Some(fault) = fault_from_chain(&chain) {
        return TransportFail::Certificate(fault);
    }
    TransportFail::Unreachable
}

fn fault_from_chain(chain: &str) -> Option<anybot_foundation::CertFault> {
    if chain.contains("NotValidForName") {
        return Some(anybot_foundation::CertFault::NameMismatch);
    }
    if chain.contains("NotValidYet") {
        return Some(anybot_foundation::CertFault::NotValidYet);
    }
    if chain.contains("Expired") {
        return Some(anybot_foundation::CertFault::Expired);
    }
    if chain.contains("certificate") || chain.contains("Certificate") {
        return Some(anybot_foundation::CertFault::Untrusted);
    }
    None
}
