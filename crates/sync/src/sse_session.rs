use std::sync::Arc;

use anybot_foundation::{Clock, Secret};
use anybot_protocol::ProblemJson;
use tokio::sync::mpsc;

use crate::heartbeat_idle::{IdleWatcher, LinkFailure};
use crate::heartbeat_keepalive::RequestActivity;
use crate::reconnect_http::SyncHttp;
use crate::reconnect_pacing::IDLE_LIMIT;
use crate::sse_hint::{Hint, HintFault, hint_of};
use crate::sse_parse::{SseItem, SseParser};

pub const NOTIFICATIONS_PATH: &str = "/v1/streams/notifications";
pub const CLIENT_VERSION_HEADER: &str = "AnyBot-Client-Version";
const VERSION_TOO_OLD_CODE: &str = "install-version-old";

pub struct SseTarget {
    pub base_url: String,
    pub credential: Secret,
    pub client_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseSignal {
    Connected,
    Hint(Hint),
    Unreadable(HintFault),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEnd {
    ConnectTimeout,
    ConnectFailed(String),
    Idle,
    Dropped(String),
    ClosedByServer,
    CredentialRejected,
    VersionTooOld,
    Rejected { status: u16 },
    ReceiverGone,
}

impl From<LinkFailure> for SseEnd {
    fn from(failure: LinkFailure) -> Self {
        match failure {
            LinkFailure::ConnectTimeout => SseEnd::ConnectTimeout,
            LinkFailure::ConnectFailed(reason) => SseEnd::ConnectFailed(reason),
            LinkFailure::Idle => SseEnd::Idle,
            LinkFailure::Broken(reason) => SseEnd::Dropped(reason),
        }
    }
}

pub async fn run_sse_session(
    http: &SyncHttp,
    clock: Arc<dyn Clock>,
    target: &SseTarget,
    activity: &RequestActivity,
    signals: &mpsc::Sender<SseSignal>,
) -> SseEnd {
    let mut watcher = IdleWatcher::start(clock, IDLE_LIMIT, Arc::clone(&http.probe));
    activity.record();
    let request = http
        .client
        .get(format!(
            "{}{NOTIFICATIONS_PATH}",
            target.base_url.trim_end_matches('/')
        ))
        .bearer_auth(target.credential.expose())
        .header(CLIENT_VERSION_HEADER, &target.client_version);
    let mut response = match watcher.send(request).await {
        Ok(response) => response,
        Err(failure) => return failure.into(),
    };
    if let Some(end) = refusal(&mut response, &watcher).await {
        return end;
    }
    if signals.send(SseSignal::Connected).await.is_err() {
        return SseEnd::ReceiverGone;
    }
    pump(&mut response, &mut watcher, signals).await
}

async fn refusal(response: &mut reqwest::Response, watcher: &IdleWatcher) -> Option<SseEnd> {
    let status = response.status();
    if status.is_success() {
        return None;
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Some(SseEnd::CredentialRejected);
    }
    if status == reqwest::StatusCode::FORBIDDEN && version_refused(response, watcher).await {
        return Some(SseEnd::VersionTooOld);
    }
    Some(SseEnd::Rejected {
        status: status.as_u16(),
    })
}

async fn version_refused(response: &mut reqwest::Response, watcher: &IdleWatcher) -> bool {
    let body = tokio::select! {
        read = response.chunk() => read,
        () = watcher.expired() => return false,
    };
    let Ok(Some(bytes)) = body else {
        return false;
    };
    serde_json::from_slice::<ProblemJson>(&bytes)
        .is_ok_and(|problem| problem.code == VERSION_TOO_OLD_CODE)
}

async fn pump(
    response: &mut reqwest::Response,
    watcher: &mut IdleWatcher,
    signals: &mpsc::Sender<SseSignal>,
) -> SseEnd {
    let mut parser = SseParser::new();
    loop {
        let chunk = match watcher.next_chunk(response).await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return SseEnd::ClosedByServer,
            Err(failure) => return failure.into(),
        };
        for item in parser.push(&chunk) {
            let SseItem::Event(event) = item else {
                continue;
            };
            let signal = match hint_of(&event) {
                Ok(Some(hint)) => SseSignal::Hint(hint),
                Ok(None) => continue,
                Err(fault) => SseSignal::Unreadable(fault),
            };
            if signals.send(signal).await.is_err() {
                return SseEnd::ReceiverGone;
            }
        }
    }
}
