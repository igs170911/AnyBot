use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::clock::Clock;
use tokio::sync::watch;

use crate::GatewayFail;
use crate::errors::{Classified, FailInput, RetryPlan, classify, clip_detail, redact};
use crate::notice::{Notice, busy_notice, failure_notice, retry_notice};
use crate::retry::{self, apply_jitter, base_wait, retry_after};
use crate::transport::TransportFail;
use crate::types::{Audience, ProviderKind, TurnResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttemptLimit {
    pub audience: Audience,
    pub jitter_milli: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AttemptFail {
    pub classified: Classified,
    pub detail: String,
    pub retry_after: Option<u64>,
    pub notices: Vec<Notice>,
}

pub(crate) async fn wait_retry(
    clock: &Arc<dyn Clock>,
    stop: &watch::Receiver<bool>,
    fail: &mut AttemptFail,
    sent: u32,
    limit: AttemptLimit,
) -> Result<bool, GatewayFail> {
    if matches!(fail.classified.retry, RetryPlan::None) {
        return Ok(false);
    }
    if sent >= retry::try_limit(limit.audience) {
        return Ok(false);
    }
    let wait = wait_for(fail, sent, limit);
    let notice = if matches!(fail.classified.retry, RetryPlan::BusyWait) {
        busy_notice()
    } else {
        retry_notice(sent, retry::try_limit(limit.audience), wait)
    };
    fail.notices.push(notice);
    if *stop.borrow() {
        return Err(GatewayFail::Stopped);
    }
    let sleep = clock.sleep(wait);
    tokio::pin!(sleep);
    let mut stop = stop.clone();
    tokio::select! {
        biased;
        () = &mut sleep => Ok(true),
        changed = stop.changed() => {
            let stopped = changed.is_ok() && *stop.borrow();
            if stopped {
                Err(GatewayFail::Stopped)
            } else {
                Ok(true)
            }
        }
    }
}

fn wait_for(fail: &AttemptFail, sent: u32, limit: AttemptLimit) -> Duration {
    if matches!(fail.classified.retry, RetryPlan::BusyWait) {
        return retry::BUSY_WAIT;
    }
    let base = base_wait(limit.audience, sent.saturating_sub(1));
    let jittered = apply_jitter(base, limit.jitter_milli, limit.audience);
    retry_after(fail.retry_after, jittered)
}

pub(crate) fn from_status(
    status: u16,
    body: &str,
    provider: ProviderKind,
    secret: Option<&str>,
    retry_after_secs: Option<u64>,
) -> AttemptFail {
    let message = extract_message(body);
    let classified = classify(&FailInput::Http {
        status,
        message: message.clone(),
    });
    let detail = redact(&clip_detail(&message), secret);
    let notice = failure_notice(&classified, provider, &detail);
    AttemptFail {
        classified,
        detail,
        retry_after: retry_after_secs,
        notices: vec![notice],
    }
}

pub(crate) fn from_transport(
    fail: &TransportFail,
    provider: ProviderKind,
    saw_text: bool,
) -> AttemptFail {
    let input = match fail {
        TransportFail::ConnectTimeout | TransportFail::Unreachable => FailInput::Connect {
            ollama: matches!(provider, ProviderKind::Ollama),
        },
        TransportFail::FirstToken => FailInput::FirstToken,
        TransportFail::Stalled { .. } => FailInput::Stalled,
        TransportFail::Partial(body) if body.stalled => FailInput::Stalled,
        TransportFail::Dropped | TransportFail::Partial(_) => FailInput::StreamCut { saw_text },
    };
    let classified = classify(&input);
    let detail = fail_label(fail);
    let notice = failure_notice(&classified, provider, &detail);
    AttemptFail {
        classified,
        detail,
        retry_after: None,
        notices: vec![notice],
    }
}

fn fail_label(fail: &TransportFail) -> String {
    match fail {
        TransportFail::ConnectTimeout => "connect timeout".to_owned(),
        TransportFail::Unreachable => "unreachable".to_owned(),
        TransportFail::FirstToken => "first token".to_owned(),
        TransportFail::Stalled { .. } => "stalled after text".to_owned(),
        TransportFail::Dropped => "dropped".to_owned(),
        TransportFail::Partial(body) if body.stalled => "stalled".to_owned(),
        TransportFail::Partial(_) => "partial drop".to_owned(),
    }
}

pub(crate) fn blank_result(result: &TurnResult) -> bool {
    result.text.trim().is_empty()
        && result.tool_calls.is_empty()
        && result.thinking.trim().is_empty()
        || result.is_blank()
}

pub(crate) fn classified_fail(classified: Classified, notices: Vec<Notice>) -> GatewayFail {
    GatewayFail::Classified {
        classified,
        notices,
    }
}

fn extract_message(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return body.to_owned();
    };
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .unwrap_or(body)
        .to_owned()
}
