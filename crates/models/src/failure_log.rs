use crate::errors::Classified;
use crate::errors_text::redact;
use crate::types::ProviderKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureLog {
    pub kind: crate::errors::FailKind,
    pub code: &'static str,
    pub http_status: Option<u16>,
    pub attempt: u32,
    pub detail: String,
}

#[must_use]
pub fn failure_log(
    classified: &Classified,
    attempt: u32,
    detail: &str,
    secret: Option<&str>,
) -> FailureLog {
    FailureLog {
        kind: classified.kind,
        code: classified.code,
        http_status: classified.http_status,
        attempt,
        detail: redact(detail, secret),
    }
}

#[must_use]
pub fn provider_name(kind: ProviderKind) -> &'static str {
    crate::errors_text::provider_label(kind)
}
