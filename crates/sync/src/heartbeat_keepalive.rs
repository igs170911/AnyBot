use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs};

pub const KEEPALIVE_EVERY: Duration = Duration::from_secs(20);

pub struct RequestActivity {
    clock: Arc<dyn Clock>,
    last_request_ms: AtomicU64,
}

impl RequestActivity {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let now = clock.now_ms().millis();
        RequestActivity {
            clock,
            last_request_ms: AtomicU64::new(now),
        }
    }

    pub fn record(&self) {
        self.last_request_ms
            .store(self.clock.now_ms().millis(), Ordering::SeqCst);
    }

    #[must_use]
    pub fn due_at(&self) -> TimestampMs {
        TimestampMs::from_millis(self.last_request_ms.load(Ordering::SeqCst))
            .saturating_add_ms(KEEPALIVE_EVERY)
    }

    pub async fn wait_due(&self) {
        loop {
            let due = self.due_at();
            if self.clock.now_ms() >= due {
                return;
            }
            self.clock.sleep_until(due).await;
        }
    }
}

pub async fn keepalive_loop<F, Fut>(activity: Arc<RequestActivity>, mut send: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    loop {
        activity.wait_due().await;
        activity.record();
        send().await;
        activity.record();
    }
}

#[cfg(test)]
#[path = "heartbeat_keepalive_tests.rs"]
mod tests;
