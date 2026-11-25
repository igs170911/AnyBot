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
    pub reason_code: &'static str,
    pub reason_params: Vec<(String, String)>,
}

#[must_use]
pub fn record(
    before_provider: ProviderKind,
    before_model: &str,
    after_provider: ProviderKind,
    after_model: &str,
    reason: SwitchReason,
    reason_code: &'static str,
    reason_params: Vec<(String, String)>,
) -> HandoffRecord {
    HandoffRecord {
        before_provider,
        before_model: before_model.to_owned(),
        after_provider,
        after_model: after_model.to_owned(),
        reason,
        reason_code,
        reason_params,
    }
}

#[must_use]
pub fn handoff_code(reason: SwitchReason) -> &'static str {
    reason_parts(reason, None).0
}

#[must_use]
pub fn reason_parts(
    reason: SwitchReason,
    kind_code: Option<&str>,
) -> (&'static str, Vec<(String, String)>) {
    match reason {
        SwitchReason::User => ("model-handoff-user", Vec::new()),
        SwitchReason::Bot => ("model-handoff-bot", Vec::new()),
        SwitchReason::Unreachable => ("model-handoff-unreachable", Vec::new()),
        SwitchReason::Fallback => (
            "model-handoff-fallback",
            vec![(
                "kind".to_owned(),
                kind_code.unwrap_or("model-other").to_owned(),
            )],
        ),
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
