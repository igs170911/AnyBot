use std::time::Duration;

use crate::errors::Classified;
use crate::types::ProviderKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub code: &'static str,
    pub params: Vec<(String, String)>,
}

#[must_use]
pub fn failure_notice(classified: &Classified, provider: ProviderKind, detail: &str) -> Notice {
    let mut params = vec![
        (
            "provider".to_owned(),
            crate::errors::provider_label(provider).to_owned(),
        ),
        ("detail".to_owned(), detail.to_owned()),
    ];
    if let Some(status) = classified.http_status {
        params.push(("status".to_owned(), status.to_string()));
    }
    if let Some(limit) = classified.limit {
        params.push(("limit".to_owned(), limit.to_string()));
    }
    Notice {
        code: classified.code,
        params,
    }
}

#[must_use]
pub fn retry_notice(attempt: u32, total: u32, wait: Duration) -> Notice {
    Notice {
        code: "model-retry-wait",
        params: vec![
            ("seconds".to_owned(), wait.as_secs().to_string()),
            ("attempt".to_owned(), attempt.to_string()),
            ("total".to_owned(), total.to_string()),
        ],
    }
}

#[must_use]
pub fn busy_notice() -> Notice {
    Notice {
        code: "model-busy-wait",
        params: Vec::new(),
    }
}

#[must_use]
pub fn skip_notice(provider: ProviderKind, bot: &str) -> Notice {
    Notice {
        code: "model-policy-skip",
        params: vec![
            (
                "provider".to_owned(),
                crate::errors::provider_label(provider).to_owned(),
            ),
            ("bot".to_owned(), bot.to_owned()),
        ],
    }
}

#[must_use]
pub fn stop_note(stop: crate::types::StopKind) -> Option<&'static str> {
    match stop {
        crate::types::StopKind::Refusal => Some("model-refusal"),
        crate::types::StopKind::Length => Some("model-length-note"),
        crate::types::StopKind::Finished
        | crate::types::StopKind::NeedsTool
        | crate::types::StopKind::Interrupted => None,
    }
}

#[must_use]
pub fn missing_key_notice(provider: ProviderKind) -> Notice {
    Notice {
        code: "model-missing-key",
        params: vec![(
            "provider".to_owned(),
            crate::errors::provider_label(provider).to_owned(),
        )],
    }
}
