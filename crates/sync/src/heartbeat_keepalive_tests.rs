use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use test_harness::{FakeClock, advance_clock, sleep_real, timeout_real};

use super::*;

const START_MS: u64 = 5_000_000;

fn activity(clock: &Arc<FakeClock>) -> Arc<RequestActivity> {
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    Arc::new(RequestActivity::new(shared))
}

async fn settle() {
    sleep_real(Duration::from_millis(150)).await;
}

async fn run_seconds(clock: &Arc<FakeClock>, seconds: u64) {
    for _second in 0..seconds {
        let _woken = advance_clock(clock, Duration::from_secs(1));
        settle().await;
    }
}

#[tokio::test]
async fn the_wait_ends_twenty_fake_seconds_after_the_last_request() {
    let clock = FakeClock::new(START_MS);
    let activity = activity(&clock);
    assert_eq!(
        activity.due_at(),
        TimestampMs::from_millis(START_MS + 20_000)
    );
    let waiting = {
        let owned = Arc::clone(&activity);
        tokio::spawn(async move { owned.wait_due().await })
    };
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_millis(19_999));
    settle().await;
    assert!(!waiting.is_finished());
    let _woken = advance_clock(&clock, Duration::from_millis(1));
    timeout_real(waiting, Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_request_in_between_pushes_the_due_time_out_again() {
    let clock = FakeClock::new(START_MS);
    let activity = activity(&clock);
    let waiting = {
        let owned = Arc::clone(&activity);
        tokio::spawn(async move { owned.wait_due().await })
    };
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_secs(15));
    activity.record();
    let _woken = advance_clock(&clock, Duration::from_secs(10));
    settle().await;
    assert!(!waiting.is_finished(), "20 s count from the new request");
    let _woken = advance_clock(&clock, Duration::from_secs(10));
    timeout_real(waiting, Duration::from_secs(5))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn with_nothing_else_going_on_one_request_goes_out_every_twenty_seconds() {
    let clock = FakeClock::new(START_MS);
    let activity = activity(&clock);
    let sent = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&sent);
    let looping = tokio::spawn(keepalive_loop(Arc::clone(&activity), move || {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }));
    settle().await;
    run_seconds(&clock, 59).await;
    assert_eq!(sent.load(Ordering::SeqCst), 2);
    run_seconds(&clock, 1).await;
    assert_eq!(sent.load(Ordering::SeqCst), 3);
    looping.abort();
}

#[tokio::test]
async fn other_requests_every_ten_seconds_mean_no_extra_keepalive() {
    let clock = FakeClock::new(START_MS);
    let activity = activity(&clock);
    let sent = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&sent);
    let looping = tokio::spawn(keepalive_loop(Arc::clone(&activity), move || {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    }));
    settle().await;
    for _round in 0..9 {
        let _woken = advance_clock(&clock, Duration::from_secs(10));
        activity.record();
        settle().await;
    }
    assert_eq!(sent.load(Ordering::SeqCst), 0);
    looping.abort();
}
