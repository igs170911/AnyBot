use anybot_foundation as _;
use anybot_models::{FailInput, FailKind, FallbackPlan, RetryPlan, classify, clip_detail, redact};
use bytes as _;
use futures_util as _;
use proptest as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use url as _;

fn sample(input: &FailInput) -> (FailKind, RetryPlan, FallbackPlan) {
    let row = classify(input);
    (row.kind, row.retry, row.fallback)
}

#[test]
fn t_models_errors_1() {
    let rows = [
        sample(&FailInput::MissingKey),
        sample(&FailInput::PolicyBlocked),
        sample(&http(401, "bad key")),
        sample(&http(402, "payment")),
        sample(&http(403, "forbidden")),
        sample(&http(404, "missing")),
        sample(&http(408, "slow")),
        sample(&http(429, "rate")),
        sample(&http(500, "boom")),
        sample(&http(503, "down")),
        sample(&http(529, "overloaded")),
        sample(&FailInput::Connect { ollama: false }),
        sample(&FailInput::FirstToken),
        sample(&FailInput::Stalled),
        sample(&FailInput::StreamCut { saw_text: false }),
        sample(&FailInput::StreamCut { saw_text: true }),
        sample(&http(400, "maximum context length is 50000 tokens")),
        sample(&http(400, "does not support tools")),
        sample(&http(400, "unsupported parameter: reasoning_effort")),
        sample(&FailInput::StreamBusy),
        sample(&FailInput::StreamOther),
        sample(&http(418, "teapot")),
    ];
    let expected = [
        (FailKind::MissingKey, RetryPlan::None, FallbackPlan::Skip),
        (FailKind::PolicyBlocked, RetryPlan::None, FallbackPlan::Skip),
        (FailKind::Auth, RetryPlan::None, FallbackPlan::Switch),
        (FailKind::Quota, RetryPlan::None, FallbackPlan::Switch),
        (FailKind::Auth, RetryPlan::None, FallbackPlan::Switch),
        (FailKind::NotFound, RetryPlan::None, FallbackPlan::None),
        (FailKind::Server, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::Busy, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::Server, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::Server, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::Server, RetryPlan::Again, FallbackPlan::Switch),
        (
            FailKind::Unreachable,
            RetryPlan::Again,
            FallbackPlan::Switch,
        ),
        (FailKind::FirstToken, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::Stalled, RetryPlan::None, FallbackPlan::None),
        (FailKind::StreamCut, RetryPlan::Again, FallbackPlan::Switch),
        (FailKind::StreamCut, RetryPlan::None, FallbackPlan::None),
        (FailKind::TooLong, RetryPlan::None, FallbackPlan::None),
        (FailKind::NoTools, RetryPlan::None, FallbackPlan::None),
        (
            FailKind::RejectedParam,
            RetryPlan::DropParam,
            FallbackPlan::None,
        ),
        (FailKind::Busy, RetryPlan::BusyWait, FallbackPlan::Switch),
        (FailKind::StreamError, RetryPlan::None, FallbackPlan::None),
        (FailKind::Other4xx, RetryPlan::None, FallbackPlan::None),
    ];
    assert_eq!(rows, expected);
    let too_long = classify(&http(400, "maximum context length is 50000 tokens"));
    assert_eq!(too_long.limit, Some(50_000));
}

fn http(status: u16, message: &str) -> FailInput {
    FailInput::Http {
        status,
        message: message.to_owned(),
    }
}

#[test]
fn three_too_long_bodies_share_limit() {
    for message in [
        "maximum context length is 50000 tokens",
        "This model's maximum context length is 50000 tokens",
        "prompt is too long: 50000 maximum",
    ] {
        let row = classify(&http(400, message));
        assert_eq!(row.kind, FailKind::TooLong);
        assert_eq!(row.limit, Some(50_000));
        assert_eq!(row.retry, RetryPlan::None);
    }
}

#[test]
fn redacts_key_from_classified_message() {
    let secret = "sk-live-secret";
    let shown = redact(&format!("bad key {secret}"), Some(secret));
    assert!(!shown.contains(secret));
    assert_eq!(clip_detail(&"x".repeat(400)).chars().count(), 300);
}
