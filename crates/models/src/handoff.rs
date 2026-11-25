use crate::types::ProviderKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchReason {
    User,
    Bot,
    Fallback,
    Unreachable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandoffRecord {
    pub before_provider: ProviderKind,
    pub before_model: String,
    pub after_provider: ProviderKind,
    pub after_model: String,
    pub reason: SwitchReason,
    pub detail: String,
}

#[must_use]
pub fn record(
    before_provider: ProviderKind,
    before_model: &str,
    after_provider: ProviderKind,
    after_model: &str,
    reason: SwitchReason,
    detail: &str,
) -> HandoffRecord {
    HandoffRecord {
        before_provider,
        before_model: before_model.to_owned(),
        after_provider,
        after_model: after_model.to_owned(),
        reason,
        detail: detail.to_owned(),
    }
}

#[must_use]
pub fn same_model(
    before: &str,
    after: &str,
    before_kind: ProviderKind,
    after_kind: ProviderKind,
) -> bool {
    before == after && before_kind == after_kind
}
