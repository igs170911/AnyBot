pub const SESSION_IDLE_MS: u64 = 90 * 24 * 3600 * 1000;
pub const SESSION_CAP: usize = 50;
pub const SESSION_TOUCH_SPAN_MS: u64 = 60_000;

#[must_use]
pub fn session_expired(last_active_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(last_active_ms) > SESSION_IDLE_MS
}

#[must_use]
pub fn touch_due(last_active_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(last_active_ms) >= SESSION_TOUCH_SPAN_MS
}

pub fn evict_indices(count: usize, last_active: &[u64], now_ms: u64) -> Result<Vec<usize>, String> {
    if count <= SESSION_CAP {
        return Ok(Vec::new());
    }
    let overflow = count
        .checked_sub(SESSION_CAP)
        .ok_or("evict overflow beyond count".to_owned())?;
    let mut order: Vec<(u64, usize)> = last_active
        .iter()
        .copied()
        .zip(0..)
        .filter(|(at, _)| !session_expired(*at, now_ms))
        .collect();
    order.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    order.truncate(overflow);
    Ok(order.into_iter().map(|(_, index)| index).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_ninety_days_expires() {
        let day_ms = 24 * 3600 * 1000;
        assert!(session_expired(0, 90 * day_ms + 1));
        assert!(!session_expired(0, 90 * day_ms));
    }

    #[test]
    fn cap_overflow_evicts_oldest_first() {
        let times = [500u64; 51];
        let mut ordered = times;
        ordered[0] = 1;
        ordered[1] = 2;
        let evicted = evict_indices(51, &ordered, 1000).unwrap();
        assert_eq!(evicted, vec![0]);
        assert_eq!(evict_indices(52, &ordered, 1000).unwrap(), vec![0, 1]);
        assert!(evict_indices(50, &times, 1000).unwrap().is_empty());
    }

    #[test]
    fn touch_writes_at_most_once_a_minute() {
        assert!(touch_due(0, 60_000));
        assert!(!touch_due(0, 59_999));
    }
}
