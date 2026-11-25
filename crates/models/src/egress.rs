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

fn judge_place_ok(policy: &DataPolicy, item: &JudgeCandidate) -> bool {
    match policy {
        DataPolicy::LocalOnly => !matches!(item.place, RunPlace::Cloud),
        DataPolicy::Cloud { allowed } => match item.place {
            RunPlace::Local | RunPlace::Lan => true,
            RunPlace::Cloud => allowed.contains(&item.provider),
        },
    }
}
