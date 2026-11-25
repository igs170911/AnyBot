use std::time::Duration;

use anybot_foundation::TimestampMs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunningSpan {
    pub start: TimestampMs,
    pub end: TimestampMs,
}

impl RunningSpan {
    #[must_use]
    pub fn new(start: TimestampMs, end: TimestampMs) -> Self {
        RunningSpan { start, end }
    }

    #[must_use]
    pub fn covers(self, at: TimestampMs) -> bool {
        self.start <= at && at <= self.end
    }
}

#[must_use]
pub fn covers_by_running(spans: &[RunningSpan], at: TimestampMs) -> bool {
    spans.iter().any(|span| span.covers(at))
}

#[must_use]
pub fn idle_span(last_activity: TimestampMs, at: TimestampMs) -> Option<Duration> {
    let delta = at.millis().checked_sub(last_activity.millis())?;
    Some(Duration::from_millis(delta))
}

#[must_use]
pub fn is_stalled(
    spans: &[RunningSpan],
    last_activity: TimestampMs,
    at: TimestampMs,
    threshold: Duration,
) -> bool {
    if covers_by_running(spans, at) {
        return false;
    }
    idle_span(last_activity, at).is_some_and(|idle| idle >= threshold)
}

#[cfg(test)]
#[path = "task_stall_tests.rs"]
mod tests;
