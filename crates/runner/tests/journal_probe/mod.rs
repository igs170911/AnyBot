use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use anybot_protocol::{ActualResult, ComputerCaps};
use anybot_store_client::Store;
use anybot_trust::{GatedComputer, OpRejection, OpTicket};
use runner_test::FakeComputer;

pub(crate) struct CrashProbe {
    pub inner: Arc<FakeComputer>,
    pub store: Store,
    pub stream: String,
    pub handle: tokio::runtime::Handle,
    pub exec_count: AtomicUsize,
    pub missing_intent: AtomicBool,
}

impl CrashProbe {
    fn check_intent(&self) {
        let count = self
            .exec_count
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let ok = tokio::task::block_in_place(|| {
            self.handle.block_on(async {
                self.store
                    .outbox_pending(&self.stream, 500)
                    .await
                    .is_ok_and(|rows| {
                        rows.iter().filter(|r| r.kind == "tool_intent").count() >= count
                    })
            })
        });
        if !ok {
            self.missing_intent.store(true, Ordering::SeqCst);
        }
    }
}

impl GatedComputer for CrashProbe {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn declared_caps(&self) -> ComputerCaps {
        self.inner.declared_caps()
    }

    fn capabilities(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.inner.capabilities(t)
    }

    fn run_command(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.check_intent();
        self.inner.run_command(t)
    }

    fn read_file(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.check_intent();
        self.inner.read_file(t)
    }

    fn write_file(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.check_intent();
        self.inner.write_file(t)
    }

    fn edit_file(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.check_intent();
        self.inner.edit_file(t)
    }

    fn path_info(&self, t: OpTicket) -> Result<ActualResult, OpRejection> {
        self.inner.path_info(t)
    }

    fn resolve_target(&self, path: &str) -> Option<String> {
        self.inner.resolve_target(path)
    }

    fn declared_workdir(&self) -> String {
        self.inner.declared_workdir()
    }

    fn gate_config(&self) -> anybot_trust::GateConfig {
        self.inner.gate_config()
    }
}

pub(crate) struct DropAfterPolls<F> {
    pub inner: Pin<Box<F>>,
    pub remaining: usize,
}

impl<F: Future> Future for DropAfterPolls<F> {
    type Output = Option<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            return Poll::Ready(None);
        }
        self.remaining = self.remaining.saturating_sub(1);
        match self.inner.as_mut().poll(cx) {
            Poll::Ready(out) => Poll::Ready(Some(out)),
            Poll::Pending => Poll::Pending,
        }
    }
}
