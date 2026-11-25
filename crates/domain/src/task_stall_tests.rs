use std::time::Duration;

use anybot_foundation::TimestampMs;

use super::{RunningSpan, is_stalled};

fn ms(v: u64) -> TimestampMs {
    TimestampMs::from_millis(v)
}

fn fifteen_minutes() -> Duration {
    Duration::from_mins(15)
}

#[test]
fn t_task_stall_4() {
    let running = [RunningSpan::new(ms(0), ms(100_000))];
    assert!(!is_stalled(
        &running,
        ms(94_000),
        ms(100_000),
        fifteen_minutes()
    ));
    assert!(!is_stalled(
        &[],
        ms(94_000),
        ms(94_000u64.saturating_add(840_000)),
        fifteen_minutes()
    ));
    assert!(is_stalled(
        &[],
        ms(94_000),
        ms(94_000u64.saturating_add(960_000)),
        fifteen_minutes()
    ));
}

#[test]
fn test_stall_never_fires_without_activity_data() {
    assert!(!is_stalled(&[], ms(50_000), ms(10_000), fifteen_minutes()));
}
