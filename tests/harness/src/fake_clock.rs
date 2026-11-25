use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;

pub const CODE_FAKE_CLOCK_NO_FUTURE: &str = "fake-clock-no-future";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockGone {
    pub code: String,
}

struct Sleeper {
    deadline_ms: u64,
    order: u64,
    wake: Arc<Notify>,
}

#[derive(Default)]
struct FakeState {
    sleepers: Vec<Sleeper>,
    next_order: u64,
}

pub struct FakeClock {
    now_ms: AtomicU64,
    state: Mutex<FakeState>,
}

impl FakeClock {
    #[must_use]
    pub fn new(start_ms: u64) -> Arc<Self> {
        Arc::new(FakeClock {
            now_ms: AtomicU64::new(start_ms),
            state: Mutex::new(FakeState::default()),
        })
    }

    #[must_use]
    pub fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn advance(&self, span: Duration) -> usize {
        let add_ms = u64::try_from(span.as_millis()).unwrap_or(u64::MAX);
        let new_now = self
            .now_ms
            .fetch_add(add_ms, Ordering::SeqCst)
            .saturating_add(add_ms);
        let due = self.take_due(new_now);
        for sleeper in &due {
            sleeper.wake.notify_one();
        }
        due.len()
    }

    fn take_due(&self, new_now: u64) -> Vec<Sleeper> {
        let Ok(mut state) = self.state.lock() else {
            return Vec::new();
        };
        let due_index = state
            .sleepers
            .iter()
            .take_while(|s| s.deadline_ms <= new_now)
            .count();
        let due: Vec<Sleeper> = state.sleepers.drain(..due_index).collect();
        due
    }

    fn register(&self, deadline_ms: u64, wake: Arc<Notify>) {
        let now = self.now_ms.load(Ordering::SeqCst);
        let due = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            let order = state.next_order;
            state.next_order = order.saturating_add(1);
            state.sleepers.push(Sleeper {
                deadline_ms,
                order,
                wake,
            });
            state.sleepers.sort_by_key(|s| (s.deadline_ms, s.order));
            let due_index = state
                .sleepers
                .iter()
                .take_while(|s| s.deadline_ms <= now)
                .count();
            state.sleepers.drain(..due_index).collect::<Vec<Sleeper>>()
        };
        for sleeper in &due {
            sleeper.wake.notify_one();
        }
    }
}

pub fn sleep_until(
    clock: &Arc<FakeClock>,
    deadline_ms: u64,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
    let owned = Arc::clone(clock);
    Box::pin(async move {
        let wake = Arc::new(Notify::new());
        FakeClock::register(&owned, deadline_ms, Arc::clone(&wake));
        wake.notified().await;
    })
}

pub fn sleep_for(
    clock: &Arc<FakeClock>,
    span: Duration,
) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
    let span_ms = u64::try_from(span.as_millis()).unwrap_or(u64::MAX);
    let deadline = clock.now_ms().saturating_add(span_ms);
    sleep_until(clock, deadline)
}

pub fn timeout<T>(
    clock: &Arc<FakeClock>,
    span: Duration,
    task: T,
) -> Pin<Box<dyn Future<Output = Result<T::Output, ClockGone>> + Send>>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let owned = Arc::clone(clock);
    Box::pin(async move {
        let expired = sleep_for(&owned, span);
        tokio::pin!(task);
        tokio::select! {
            biased;
            outcome = &mut task => Ok(outcome),
            _elapsed = expired => Err(ClockGone { code: CODE_FAKE_CLOCK_NO_FUTURE.to_owned() }),
        }
    })
}

pub fn advance_clock(clock: &Arc<FakeClock>, span: Duration) -> usize {
    clock.advance(span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_clock_wakes_sleepers_in_deadline_order() {
        let clock = Arc::new(FakeClock::new(1_000));
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_a = Arc::clone(&log);
        let clock_a = Arc::clone(&clock);
        let first = tokio::spawn(async move {
            sleep_for(&clock_a, Duration::from_millis(30)).await;
            log_a.lock().unwrap().push("short");
        });
        let log_b = Arc::clone(&log);
        let clock_b = Arc::clone(&clock);
        let second = tokio::spawn(async move {
            sleep_for(&clock_b, Duration::from_millis(60)).await;
            log_b.lock().unwrap().push("long");
        });
        tokio::task::yield_now().await;
        let _woken = clock.advance(Duration::from_millis(60));
        let first_done = crate::realtime::timeout_real(first, Duration::from_secs(2)).await;
        let second_done = crate::realtime::timeout_real(second, Duration::from_secs(2)).await;
        first_done.unwrap().unwrap();
        second_done.unwrap().unwrap();
        assert_eq!(*log.lock().unwrap(), vec!["short", "long"]);
    }

    #[tokio::test]
    async fn test_fake_clock_timeout_fires_after_advance() {
        let clock = Arc::new(FakeClock::new(1_000));
        let waiting = tokio::spawn(timeout(
            &clock,
            Duration::from_millis(50),
            std::future::pending::<u8>(),
        ));
        tokio::task::yield_now().await;
        let _woken = clock.advance(Duration::from_millis(50));
        let bounded = crate::realtime::timeout_real(waiting, Duration::from_secs(2)).await;
        let outcome = bounded.unwrap().unwrap();
        assert_eq!(
            outcome,
            Err(ClockGone {
                code: CODE_FAKE_CLOCK_NO_FUTURE.to_owned()
            })
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fake_clock_timeout_wins_when_task_completes() {
        let clock = Arc::new(FakeClock::new(1_000));
        let outcome = crate::realtime::timeout_real(
            timeout(&clock, Duration::from_secs(9), std::future::ready(4u8)),
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        assert_eq!(outcome, Ok(4u8));
    }

    #[tokio::test]
    async fn test_fake_clock_advance_reports_woken_count() {
        let clock = Arc::new(FakeClock::new(1_000));
        let a = tokio::spawn(sleep_until(&clock, 1_040));
        let b = tokio::spawn(sleep_until(&clock, 1_070));
        tokio::task::yield_now().await;
        let woken = clock.advance(Duration::from_millis(50));
        assert_eq!(woken, 1);
        crate::realtime::timeout_real(a, Duration::from_secs(2))
            .await
            .unwrap()
            .unwrap();
        tokio::task::yield_now().await;
        let woken = clock.advance(Duration::from_millis(50));
        assert_eq!(woken, 1);
        crate::realtime::timeout_real(b, Duration::from_secs(2))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(clock.now_ms(), 1_100);
    }
}
