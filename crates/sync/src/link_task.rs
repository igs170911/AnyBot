use std::sync::Arc;

use anybot_store_client::ACCOUNT_STREAM;
use tokio::sync::mpsc;

use crate::core::Core;
use crate::heartbeat_keepalive::keepalive_loop;
use crate::hooks::{StreamMark, SyncHooks};
use crate::reconnect_http::SyncHttp;
use crate::sse_hint::{self, Hint};
use crate::sse_session::{SseEnd, SseSignal, SseTarget, run_sse_session};
use crate::status::ConnectionState;

const SIGNAL_QUEUE: usize = 64;

pub(crate) struct LinkParts {
    pub http: SyncHttp,
    pub target: SseTarget,
}

pub(crate) async fn run_link(core: Arc<Core>, parts: LinkParts) {
    let hooks = SyncHooks::new(Arc::clone(&core));
    loop {
        if core.halted() {
            core.link.retry.notified().await;
            continue;
        }
        core.link.wait_turn().await;
        core.link.board.connecting();
        let end = one_session(&core, &hooks, &parts).await;
        core.link.set_sse_live(false);
        settle_end(&core, &hooks, &end);
    }
}

async fn one_session(core: &Arc<Core>, hooks: &SyncHooks, parts: &LinkParts) -> SseEnd {
    let (sender, mut signals) = mpsc::channel(SIGNAL_QUEUE);
    let clock = Arc::clone(&core.clock);
    let session = run_sse_session(
        &parts.http,
        clock,
        &parts.target,
        &core.link.activity,
        &sender,
    );
    tokio::pin!(session);
    loop {
        tokio::select! {
            end = &mut session => {
                while let Ok(signal) = signals.try_recv() {
                    handle(core, hooks, signal).await;
                }
                return end;
            }
            Some(signal) = signals.recv() => handle(core, hooks, signal).await,
        }
    }
}

async fn handle(core: &Arc<Core>, hooks: &SyncHooks, signal: SseSignal) {
    let outcome = match signal {
        SseSignal::Connected => {
            core.link.set_sse_live(true);
            core.link.board.connected();
            hooks.connection_gained();
            crate::client::want_everything_known(core).await
        }
        SseSignal::Hint(Hint::Progress { streams }) => {
            let marks: Vec<StreamMark> = streams.into_iter().map(mark_of).collect();
            hooks.server_progress(&marks).await
        }
        SseSignal::Hint(Hint::News(mark)) => hooks.server_has_news(&mark_of(mark)).await,
        SseSignal::Unreadable(_fault) => Ok(()),
    };
    if let Err(error) = outcome {
        let text = error.to_string();
        core.status.update(|status| status.local_error = Some(text));
    }
}

fn mark_of(mark: sse_hint::StreamMark) -> StreamMark {
    StreamMark {
        stream: mark.stream,
        seq: mark.seq,
        record_id: mark.device_id,
    }
}

fn settle_end(core: &Core, hooks: &SyncHooks, end: &SseEnd) {
    match end {
        SseEnd::CredentialRejected => core.halt(ConnectionState::NeedLogin),
        SseEnd::VersionTooOld => core.halt(ConnectionState::VersionTooOld),
        SseEnd::ConnectTimeout
        | SseEnd::ConnectFailed(_)
        | SseEnd::Idle
        | SseEnd::Dropped(_)
        | SseEnd::ClosedByServer
        | SseEnd::Rejected { .. }
        | SseEnd::ReceiverGone => {
            if let Err(error) = hooks.connection_lost(&end_text(end)) {
                let text = error.to_string();
                core.status.update(|status| status.local_error = Some(text));
            }
        }
    }
    let pacer = &core.link.pacer;
    core.link
        .board
        .ended(end, pacer.retry_at(), pacer.failures_in_row());
}

fn end_text(end: &SseEnd) -> String {
    match end {
        SseEnd::ConnectTimeout => "connect-timeout".to_owned(),
        SseEnd::ConnectFailed(reason) | SseEnd::Dropped(reason) => reason.clone(),
        SseEnd::Idle => "idle".to_owned(),
        SseEnd::ClosedByServer => "closed-by-server".to_owned(),
        SseEnd::Rejected { status } => format!("rejected-{status}"),
        SseEnd::CredentialRejected => "credential-rejected".to_owned(),
        SseEnd::VersionTooOld => "version-too-old".to_owned(),
        SseEnd::ReceiverGone => "receiver-gone".to_owned(),
    }
}

pub(crate) async fn run_keepalive(core: Arc<Core>) {
    let activity = Arc::clone(&core.link.activity);
    keepalive_loop(activity, || {
        let core = Arc::clone(&core);
        async move {
            if core.link.sse_live() {
                core.wanted.pull(ACCOUNT_STREAM);
                core.wake.notify_one();
            }
        }
    })
    .await;
}

pub(crate) async fn run_status_mirror(core: Arc<Core>) {
    let mut watcher = core.link.board.subscribe();
    loop {
        let seen = *watcher.borrow_and_update();
        core.status.update(|status| status.link = Some(seen));
        if watcher.changed().await.is_err() {
            return;
        }
    }
}

pub(crate) async fn run_slowness(core: Arc<Core>) {
    core.link.board.watch_slowness().await;
}
