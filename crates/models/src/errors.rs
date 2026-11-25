#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailKind {
    MissingKey,
    PolicyBlocked,
    Auth,
    Quota,
    Busy,
    Server,
    Unreachable,
    FirstToken,
    Stalled,
    StreamCut,
    TooLong,
    NoTools,
    RejectedParam,
    NotFound,
    StreamError,
    Other4xx,
    Redirect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryPlan {
    None,
    Again,
    BusyWait,
    DropParam,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallbackPlan {
    None,
    Skip,
    Switch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classified {
    pub kind: FailKind,
    pub retry: RetryPlan,
    pub fallback: FallbackPlan,
    pub code: &'static str,
    pub http_status: Option<u16>,
    pub limit: Option<u64>,
    pub retry_after: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailInput {
    MissingKey,
    PolicyBlocked,
    Http { status: u16, message: String },
    Unreachable,
    FirstToken,
    Stalled,
    StreamCut { saw_text: bool },
    StreamBusy,
    StreamOther,
    RejectedParam,
    Redirect,
    Budget,
    Connect { ollama: bool },
}

#[must_use]
pub fn classify(input: &FailInput) -> Classified {
    match input {
        FailInput::MissingKey => skip_row(FailKind::MissingKey, "model-missing-key"),
        FailInput::PolicyBlocked => skip_row(FailKind::PolicyBlocked, "model-policy-blocked"),
        FailInput::Unreachable => again_row(FailKind::Unreachable, "model-unreachable"),
        FailInput::Connect { ollama } => connect_row(*ollama),
        FailInput::FirstToken => again_row(FailKind::FirstToken, "model-first-token"),
        FailInput::Stalled => none_row(FailKind::Stalled, "model-stalled"),
        FailInput::StreamCut { saw_text } => stream_cut(*saw_text),
        FailInput::StreamBusy => plain(
            FailKind::Busy,
            RetryPlan::BusyWait,
            FallbackPlan::Switch,
            "model-busy",
        ),
        FailInput::StreamOther => plain(
            FailKind::StreamError,
            RetryPlan::None,
            FallbackPlan::None,
            "model-stream-error",
        ),
        FailInput::RejectedParam => plain(
            FailKind::RejectedParam,
            RetryPlan::DropParam,
            FallbackPlan::None,
            "model-rejected-param",
        ),
        FailInput::Redirect => plain(
            FailKind::Redirect,
            RetryPlan::None,
            FallbackPlan::None,
            "model-redirect",
        ),
        FailInput::Budget => plain(
            FailKind::Other4xx,
            RetryPlan::None,
            FallbackPlan::None,
            "model-budget",
        ),
        FailInput::Http { status, message } => classify_http(*status, message),
    }
}

fn connect_row(ollama: bool) -> Classified {
    let code = if ollama {
        "model-ollama-down"
    } else {
        "model-unreachable"
    };
    again_row(FailKind::Unreachable, code)
}

fn stream_cut(saw_text: bool) -> Classified {
    if saw_text {
        plain(
            FailKind::StreamCut,
            RetryPlan::None,
            FallbackPlan::None,
            "model-stream-cut",
        )
    } else {
        plain(
            FailKind::StreamCut,
            RetryPlan::Again,
            FallbackPlan::Switch,
            "model-stream-cut",
        )
    }
}

fn classify_http(status: u16, message: &str) -> Classified {
    if let Some(found) = body_kind(status, message) {
        return found;
    }
    status_kind(status)
}

fn body_kind(status: u16, message: &str) -> Option<Classified> {
    if too_long(message) {
        return Some(with_limit(
            FailKind::TooLong,
            RetryPlan::None,
            FallbackPlan::None,
            "model-too-long",
            status,
            context_limit(message),
        ));
    }
    if no_tools(message) {
        return Some(with_status(
            FailKind::NoTools,
            RetryPlan::None,
            FallbackPlan::None,
            "model-no-tools",
            status,
        ));
    }
    if rejected_param(message) {
        return Some(with_status(
            FailKind::RejectedParam,
            RetryPlan::DropParam,
            FallbackPlan::None,
            "model-rejected-param",
            status,
        ));
    }
    if quota(status, message) {
        return Some(with_status(
            FailKind::Quota,
            RetryPlan::None,
            FallbackPlan::Switch,
            "model-quota",
            status,
        ));
    }
    None
}

fn status_kind(status: u16) -> Classified {
    match status {
        401 | 403 => auth_row(status),
        404 => with_status(
            FailKind::NotFound,
            RetryPlan::None,
            FallbackPlan::None,
            "model-not-found",
            status,
        ),
        429 => with_status(
            FailKind::Busy,
            RetryPlan::Again,
            FallbackPlan::Switch,
            "model-rate-limit",
            status,
        ),
        408 | 529 | 500..=599 => server_row(status),
        400..=499 => with_status(
            FailKind::Other4xx,
            RetryPlan::None,
            FallbackPlan::None,
            "model-other",
            status,
        ),
        _ => server_row(status),
    }
}

fn auth_row(status: u16) -> Classified {
    with_status(
        FailKind::Auth,
        RetryPlan::None,
        FallbackPlan::Switch,
        "model-auth",
        status,
    )
}

fn server_row(status: u16) -> Classified {
    with_status(
        FailKind::Server,
        RetryPlan::Again,
        FallbackPlan::Switch,
        "model-server",
        status,
    )
}

fn skip_row(kind: FailKind, code: &'static str) -> Classified {
    plain(kind, RetryPlan::None, FallbackPlan::Skip, code)
}

fn again_row(kind: FailKind, code: &'static str) -> Classified {
    plain(kind, RetryPlan::Again, FallbackPlan::Switch, code)
}

fn none_row(kind: FailKind, code: &'static str) -> Classified {
    plain(kind, RetryPlan::None, FallbackPlan::None, code)
}

fn plain(
    kind: FailKind,
    retry: RetryPlan,
    fallback: FallbackPlan,
    code: &'static str,
) -> Classified {
    row(kind, retry, fallback, code, None, None)
}

fn with_status(
    kind: FailKind,
    retry: RetryPlan,
    fallback: FallbackPlan,
    code: &'static str,
    status: u16,
) -> Classified {
    row(kind, retry, fallback, code, Some(status), None)
}

fn with_limit(
    kind: FailKind,
    retry: RetryPlan,
    fallback: FallbackPlan,
    code: &'static str,
    status: u16,
    limit: Option<u64>,
) -> Classified {
    row(kind, retry, fallback, code, Some(status), limit)
}

fn row(
    kind: FailKind,
    retry: RetryPlan,
    fallback: FallbackPlan,
    code: &'static str,
    http_status: Option<u16>,
    limit: Option<u64>,
) -> Classified {
    Classified {
        kind,
        retry,
        fallback,
        code,
        http_status,
        limit,
        retry_after: None,
    }
}

use crate::errors_text::{context_limit, no_tools, quota, rejected_param, too_long};

pub use crate::errors_text::{
    clip_detail, context_limit as context_limit_of, provider_label, redact,
};
