pub const WINDOW_SPAN_MS: u64 = 300_000;

#[must_use]
pub fn count_in_window(events: &[u64], now_ms: u64, span_ms: u64) -> usize {
    let Some(floor) = now_ms.checked_sub(span_ms) else {
        return events.len();
    };
    events.iter().filter(|at| **at > floor && **at <= now_ms).count()
}

#[must_use]
pub fn five_minute_count(events: &[u64], now_ms: u64) -> usize {
    count_in_window(events, now_ms, WINDOW_SPAN_MS)
}

#[must_use]
pub fn would_exceed(events: &[u64], now_ms: u64, span_ms: u64, limit: usize) -> bool {
    count_in_window(events, now_ms, span_ms) >= limit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_counts_only_recent_events() {
        let events = [100, 200_000, 299_999];
        assert_eq!(count_in_window(&events, 300_000, 300_000), 2);
        assert_eq!(five_minute_count(&events, 300_000), 2);
    }

    #[test]
    fn limit_flag_hits_at_the_limit() {
        let events = [1, 2, 3, 4, 5];
        assert!(would_exceed(&events, 10, WINDOW_SPAN_MS, 5));
        assert!(!would_exceed(&events, 10, WINDOW_SPAN_MS, 6));
    }
}
