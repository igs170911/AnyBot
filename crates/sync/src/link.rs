use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anybot_foundation::Clock;
use tokio::sync::Notify;

use crate::core::Core;
use crate::heartbeat_keepalive::RequestActivity;
use crate::reconnect_pacing::ReconnectPacer;
use crate::reconnect_random::{RandomSource, RandomUnavailable};
use crate::reconnect_status::StatusBoard;
use crate::wire::{WireFailure, WireReply, WireRequest};

pub(crate) struct Link {
    pub pacer: ReconnectPacer,
    pub board: Arc<StatusBoard>,
    pub activity: Arc<RequestActivity>,
    pub retry: Notify,
    clock: Arc<dyn Clock>,
    sse_live: AtomicBool,
}

impl Link {
    pub(crate) fn new(clock: &Arc<dyn Clock>, random: Arc<dyn RandomSource>) -> Self {
        Link {
            pacer: ReconnectPacer::new(Arc::clone(clock), random),
            board: Arc::new(StatusBoard::new(Arc::clone(clock))),
            activity: Arc::new(RequestActivity::new(Arc::clone(clock))),
            retry: Notify::new(),
            clock: Arc::clone(clock),
            sse_live: AtomicBool::new(false),
        }
    }

    pub(crate) fn waiting(&self) -> bool {
        let now = self.clock.now_ms();
        self.pacer.retry_at().is_some_and(|at| at > now)
    }

    pub(crate) fn failed(&self) -> Result<(), RandomUnavailable> {
        if self.waiting() {
            return Ok(());
        }
        self.pacer.failed().map(|_delay| ())
    }

    pub(crate) fn set_sse_live(&self, live: bool) {
        self.sse_live.store(live, Ordering::SeqCst);
    }

    pub(crate) fn sse_live(&self) -> bool {
        self.sse_live.load(Ordering::SeqCst)
    }

    pub(crate) async fn wait_turn(&self) {
        let retry = self.retry.notified();
        tokio::pin!(retry);
        retry.as_mut().enable();
        tokio::select! {
            () = self.pacer.wait_turn() => {}
            () = retry => {}
        }
    }
}

impl Core {
    pub(crate) async fn call(&self, request: WireRequest) -> Result<WireReply, WireFailure> {
        self.link.activity.record();
        let guard = self.link.board.request_started();
        self.wire.call(request, &guard).await
    }
}
