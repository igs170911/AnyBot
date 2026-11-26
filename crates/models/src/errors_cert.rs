use crate::errors::{Classified, FailKind, again_row};

pub(crate) fn certificate_row(fault: &anybot_foundation::CertFault) -> Classified {
    again_row(FailKind::Unreachable, cert_code(fault))
}

fn cert_code(fault: &anybot_foundation::CertFault) -> &'static str {
    match fault {
        anybot_foundation::CertFault::NameMismatch => "model-cert-name",
        anybot_foundation::CertFault::Expired | anybot_foundation::CertFault::NotValidYet => {
            "model-cert-expired"
        }
        anybot_foundation::CertFault::Untrusted | anybot_foundation::CertFault::Other(_) => {
            "model-cert-untrusted"
        }
    }
}
