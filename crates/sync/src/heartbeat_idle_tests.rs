use std::sync::Arc;
use std::time::Duration;

use test_harness::{FakeClock, advance_clock, sleep_real, timeout_real};

use super::*;

const START_MS: u64 = 1_000_000;

fn watcher(clock: &Arc<FakeClock>) -> (IdleWatcher, Arc<ConnectProbe>) {
    let probe = Arc::new(ConnectProbe::default());
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let watcher = IdleWatcher::start(shared, Duration::from_secs(30), Arc::clone(&probe));
    (watcher, probe)
}

async fn settle() {
    sleep_real(Duration::from_millis(150)).await;
}

#[tokio::test]
async fn silence_expires_exactly_thirty_fake_seconds_after_the_start() {
    let clock = FakeClock::new(START_MS);
    let (watcher, _probe) = watcher(&clock);
    let waiting = tokio::spawn(async move { watcher.expired().await });
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_millis(29_999));
    settle().await;
    assert!(!waiting.is_finished());
    let _woken = advance_clock(&clock, Duration::from_millis(1));
    timeout_real(waiting, Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn bytes_arriving_move_the_deadline_to_thirty_seconds_after_them() {
    let clock = FakeClock::new(START_MS);
    let (mut watcher, _probe) = watcher(&clock);
    let _woken = advance_clock(&clock, Duration::from_secs(20));
    watcher.touch();
    assert_eq!(
        watcher.deadline(),
        TimestampMs::from_millis(START_MS + 50_000)
    );
    let waiting = tokio::spawn(async move { watcher.expired().await });
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_secs(29));
    settle().await;
    assert!(
        !waiting.is_finished(),
        "thirty seconds count from the bytes"
    );
    let _woken = advance_clock(&clock, Duration::from_secs(1));
    timeout_real(waiting, Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_connect_finishing_late_restarts_the_window_while_a_sleeper_is_waiting() {
    let clock = FakeClock::new(START_MS);
    let (watcher, probe) = watcher(&clock);
    let waiting = tokio::spawn(async move { watcher.expired().await });
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_secs(4));
    probe.mark(TimestampMs::from_millis(START_MS + 4_000));
    let _woken = advance_clock(&clock, Duration::from_secs(26));
    settle().await;
    assert!(!waiting.is_finished(), "window starts at the connect");
    let _woken = advance_clock(&clock, Duration::from_secs(4));
    timeout_real(waiting, Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn a_connect_mark_older_than_this_watcher_is_ignored() {
    let clock = FakeClock::new(START_MS);
    let probe = Arc::new(ConnectProbe::default());
    probe.mark(TimestampMs::from_millis(START_MS - 10_000));
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(&clock);
    let watcher = IdleWatcher::start(shared, Duration::from_secs(30), probe);
    assert_eq!(
        watcher.deadline(),
        TimestampMs::from_millis(START_MS + 30_000)
    );
}
