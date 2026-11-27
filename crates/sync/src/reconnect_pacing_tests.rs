use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anybot_foundation::{Clock, PinBoxFuture, TimestampMs};

use super::*;
use crate::reconnect_random::SeededRandom;

struct ManualClock {
    now: AtomicU64,
}

impl Clock for ManualClock {
    fn now_ms(&self) -> TimestampMs {
        TimestampMs::from_millis(self.now.load(Ordering::SeqCst))
    }

    fn sleep(&self, _span: Duration) -> PinBoxFuture<'static, ()> {
        Box::pin(std::future::pending())
    }

    fn sleep_until(&self, _deadline: TimestampMs) -> PinBoxFuture<'static, ()> {
        Box::pin(std::future::pending())
    }
}

fn pacer_at(start: u64, seed: u64) -> (Arc<ManualClock>, ReconnectPacer) {
    let clock = Arc::new(ManualClock {
        now: AtomicU64::new(start),
    });
    let shared: Arc<dyn Clock> = Arc::<ManualClock>::clone(&clock);
    let pacer = ReconnectPacer::new(shared, Arc::new(SeededRandom::new(seed)));
    (clock, pacer)
}

fn delay_ms(pacer: &ReconnectPacer) -> u64 {
    u64::try_from(pacer.failed().unwrap().as_millis()).unwrap()
}

#[test]
fn failures_in_a_row_wait_1_2_4_8_16_30_30_seconds_plus_half_to_two_seconds() {
    let (_clock, pacer) = pacer_at(1_000, 3);
    let bases = [
        1_000u64, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000, 30_000,
    ];
    for (round, base) in bases.iter().enumerate() {
        let delay = delay_ms(&pacer);
        assert!(
            (base + 500..=base + 2_000).contains(&delay),
            "round {round}: base {base} delay {delay}"
        );
    }
}

#[test]
fn a_connection_that_lasted_over_30_seconds_restarts_from_one_second() {
    let (clock, pacer) = pacer_at(1_000, 4);
    for _round in 0..4 {
        let _delay = delay_ms(&pacer);
    }
    pacer.connected();
    clock.now.fetch_add(31_000, Ordering::SeqCst);
    let delay = delay_ms(&pacer);
    assert!((1_500..=3_000).contains(&delay), "delay {delay}");
    assert_eq!(pacer.failures_in_row(), 1);
}

#[test]
fn a_connection_shorter_than_30_seconds_keeps_the_old_step() {
    let (clock, pacer) = pacer_at(1_000, 5);
    for _round in 0..3 {
        let _delay = delay_ms(&pacer);
    }
    pacer.connected();
    clock.now.fetch_add(29_000, Ordering::SeqCst);
    let delay = delay_ms(&pacer);
    assert!((8_500..=10_000).contains(&delay), "delay {delay}");
}

#[test]
fn retry_time_counts_from_the_moment_the_failure_was_judged() {
    let (clock, pacer) = pacer_at(50_000, 6);
    let delay = delay_ms(&pacer);
    assert_eq!(
        pacer.retry_at(),
        Some(TimestampMs::from_millis(50_000 + delay))
    );
    clock.now.fetch_add(10_000, Ordering::SeqCst);
    let later = delay_ms(&pacer);
    assert_eq!(
        pacer.retry_at(),
        Some(TimestampMs::from_millis(60_000 + later))
    );
}

#[test]
fn a_response_clears_the_wait_and_only_the_first_response_starts_the_connection() {
    let (clock, pacer) = pacer_at(1_000, 7);
    let _delay = delay_ms(&pacer);
    assert!(pacer.retry_at().is_some());
    pacer.connected();
    assert!(pacer.retry_at().is_none());
    assert!(pacer.is_connected());
    clock.now.fetch_add(40_000, Ordering::SeqCst);
    pacer.connected();
    let delay = delay_ms(&pacer);
    assert!(
        (1_500..=3_000).contains(&delay),
        "stability counts from the first response: {delay}"
    );
}

#[tokio::test]
async fn waiting_for_the_turn_ends_at_once_when_a_response_arrives() {
    let (_clock, pacer) = pacer_at(1_000, 8);
    let pacer = Arc::new(pacer);
    let _delay = delay_ms(&pacer);
    let waiting = Arc::clone(&pacer);
    let handle = tokio::spawn(async move { waiting.wait_turn().await });
    tokio::task::yield_now().await;
    pacer.connected();
    let done = test_harness::timeout_real(handle, Duration::from_secs(5)).await;
    assert!(done.is_ok(), "wait_turn must end when a response arrives");
}

#[test]
fn a_thousand_clients_failing_together_never_put_more_than_15_percent_in_any_100_ms() {
    let mut delays: Vec<u64> = (0..1_000u64)
        .map(|seed| {
            let (_clock, pacer) = pacer_at(1_000, seed);
            pacer.connected();
            delay_ms(&pacer)
        })
        .collect();
    delays.sort_unstable();
    let busiest = delays
        .iter()
        .enumerate()
        .map(|(first, start)| {
            delays
                .iter()
                .skip(first)
                .take_while(|at| **at < start + 100)
                .count()
        })
        .max()
        .unwrap();
    assert!(
        busiest <= 150,
        "busiest 100 ms window held {busiest} of 1000"
    );
    assert!(delays.iter().all(|delay| (1_500..=3_000).contains(delay)));
}
