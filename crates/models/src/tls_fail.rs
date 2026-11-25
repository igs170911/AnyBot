use crate::transport::TransportFail;

pub(crate) fn map_reqwest(err: &reqwest::Error) -> TransportFail {
    match anybot_foundation::classify_tls_error(err) {
        Some(fault) => TransportFail::Certificate(fault),
        None => TransportFail::Unreachable,
    }
}
