use crate::types::ProviderKind;
use crate::where_run::{RunPlace, classify_url};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataPolicy {
    LocalOnly,
    Cloud { allowed: Vec<ProviderKind> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelUse {
    Bot,
    Fallback,
    Judge,
    Summary,
    Embed,
    Rerank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCandidate {
    pub provider: ProviderKind,
    pub has_key: bool,
    pub place: RunPlace,
}

#[must_use]
pub fn default_policy(model_url: &str, provider: ProviderKind) -> DataPolicy {
    match classify_url(model_url) {
        RunPlace::Local | RunPlace::Lan => DataPolicy::LocalOnly,
        RunPlace::Cloud => DataPolicy::Cloud {
            allowed: vec![provider],
        },
    }
}

#[must_use]
pub fn allowed(policy: &DataPolicy, url: &str, provider: ProviderKind, use_kind: ModelUse) -> bool {
    let place = classify_url(url);
    if local_only_use(use_kind) {
        return !matches!(place, RunPlace::Cloud);
    }
    match policy {
        DataPolicy::LocalOnly => !matches!(place, RunPlace::Cloud),
        DataPolicy::Cloud { allowed } => match place {
            RunPlace::Local | RunPlace::Lan => true,
            RunPlace::Cloud => allowed.contains(&provider),
        },
    }
}

fn local_only_use(use_kind: ModelUse) -> bool {
    matches!(
        use_kind,
        ModelUse::Summary | ModelUse::Embed | ModelUse::Rerank
    )
}

#[must_use]
pub fn pick_judge<'a>(
    policy: &DataPolicy,
    bot: ProviderKind,
    candidates: &'a [JudgeCandidate],
) -> Option<&'a JudgeCandidate> {
    let other = candidates.iter().find(|item| {
        item.provider != bot
            && item.has_key
            && judge_place_ok(policy, item)
            && !matches!(item.provider, ProviderKind::Ollama)
    });
    if other.is_some() {
        return other;
    }
    candidates
        .iter()
        .find(|item| item.provider == bot && item.has_key)
}

pub struct SendPermit {
    destination: String,
    use_kind: ModelUse,
    policy: DataPolicy,
}

impl std::fmt::Debug for SendPermit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SendPermit")
            .field("destination", &self.destination)
            .field("use_kind", &self.use_kind)
            .field("policy", &self.policy)
            .finish()
    }
}

impl SendPermit {
    #[must_use]
    pub fn for_endpoint(
        policy: &DataPolicy,
        endpoint: &crate::types::ProviderEndpoint,
        use_kind: ModelUse,
    ) -> Option<Self> {
        Self::issue(
            policy,
            &crate::client::chat_url_of(endpoint),
            endpoint.kind,
            use_kind,
        )
    }

    #[must_use]
    pub fn issue(
        policy: &DataPolicy,
        destination: &str,
        provider: ProviderKind,
        use_kind: ModelUse,
    ) -> Option<Self> {
        if !allowed(policy, destination, provider, use_kind) {
            return None;
        }
        Some(Self {
            destination: destination.to_owned(),
            use_kind,
            policy: policy.clone(),
        })
    }

    pub(crate) fn matches(&self, destination: &str, use_kind: ModelUse) -> bool {
        self.destination == destination && self.use_kind == use_kind
    }

    pub(crate) fn allows_url(&self, url: &str, provider: ProviderKind) -> bool {
        allowed(&self.policy, url, provider, self.use_kind)
    }

    #[must_use]
    pub(crate) fn use_kind(&self) -> ModelUse {
        self.use_kind
    }
}

fn judge_place_ok(policy: &DataPolicy, item: &JudgeCandidate) -> bool {
    match policy {
        DataPolicy::LocalOnly => !matches!(item.place, RunPlace::Cloud),
        DataPolicy::Cloud { allowed } => match item.place {
            RunPlace::Local | RunPlace::Lan => true,
            RunPlace::Cloud => allowed.contains(&item.provider),
        },
    }
}
