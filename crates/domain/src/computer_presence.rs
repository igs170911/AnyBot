pub const ONLINE_WINDOW_MS: u64 = 60_000;
pub const SEEN_WRITE_SPAN_MS: u64 = 10_000;
pub const SWEEP_SPAN_MS: u64 = 5_000;

pub const REASON_NO_SIGNAL: &str = "computers-offline-no-signal";
pub const REASON_REMOVED: &str = "computers-offline-removed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineReason {
    NoSignal,
    Removed,
}

impl OfflineReason {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            OfflineReason::NoSignal => REASON_NO_SIGNAL,
            OfflineReason::Removed => REASON_REMOVED,
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<OfflineReason> {
        match code {
            REASON_NO_SIGNAL => Some(OfflineReason::NoSignal),
            REASON_REMOVED => Some(OfflineReason::Removed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceInput {
    pub stored_last_seen_ms: u64,
    pub seen_since_start_ms: Option<u64>,
    pub started_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Online,
    Offline {
        at_ms: u64,
        reason: Option<OfflineReason>,
    },
}

#[must_use]
pub fn presence_of(now_ms: u64, input: &PresenceInput) -> Presence {
    if let Some(seen) = input.seen_since_start_ms {
        if now_ms.saturating_sub(seen) > ONLINE_WINDOW_MS {
            return Presence::Offline {
                at_ms: seen,
                reason: Some(OfflineReason::NoSignal),
            };
        }
        return Presence::Online;
    }
    let after_signal = input.stored_last_seen_ms.saturating_add(ONLINE_WINDOW_MS);
    let after_start = input.started_ms.saturating_add(ONLINE_WINDOW_MS);
    if now_ms > after_signal.max(after_start) {
        return Presence::Offline {
            at_ms: input.stored_last_seen_ms,
            reason: None,
        };
    }
    Presence::Online
}

#[must_use]
pub fn write_due(written_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(written_ms) >= SEEN_WRITE_SPAN_MS
}

pub const COMPUTER_CAP: usize = 100;

#[must_use]
pub fn evict_oldest_seen(last_seen: &[u64]) -> Option<usize> {
    if last_seen.len() < COMPUTER_CAP {
        return None;
    }
    last_seen
        .iter()
        .enumerate()
        .min_by_key(|(index, at)| (**at, *index))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(stored: u64, seen: Option<u64>, started: u64) -> PresenceInput {
        PresenceInput {
            stored_last_seen_ms: stored,
            seen_since_start_ms: seen,
            started_ms: started,
        }
    }

    #[test]
    fn sixty_seconds_after_the_last_request_is_still_online() {
        let seen = input(0, Some(1_000), 0);
        assert_eq!(presence_of(60_999, &seen), Presence::Online);
        assert_eq!(presence_of(61_000, &seen), Presence::Online);
        assert_eq!(
            presence_of(61_001, &seen),
            Presence::Offline {
                at_ms: 1_000,
                reason: Some(OfflineReason::NoSignal),
            }
        );
    }

    #[test]
    fn after_a_restart_the_grace_covers_sixty_seconds_without_a_reason() {
        let quiet = input(5_000, None, 500_000);
        assert_eq!(presence_of(559_999, &quiet), Presence::Online);
        assert_eq!(presence_of(560_000, &quiet), Presence::Online);
        assert_eq!(
            presence_of(560_001, &quiet),
            Presence::Offline {
                at_ms: 5_000,
                reason: None,
            }
        );
    }

    #[test]
    fn a_signal_just_before_the_restart_extends_the_grace() {
        let quiet = input(499_000, None, 500_000);
        assert_eq!(presence_of(560_000, &quiet), Presence::Online);
        assert!(matches!(
            presence_of(560_001, &quiet),
            Presence::Offline { at_ms: 499_000, .. }
        ));
    }

    #[test]
    fn writes_are_spaced_by_the_throttle() {
        assert!(!write_due(1_000, 10_999));
        assert!(write_due(1_000, 11_000));
    }

    #[test]
    fn a_full_account_evicts_the_longest_silent_computer() {
        let mut seen = vec![500u64; COMPUTER_CAP - 1];
        assert_eq!(evict_oldest_seen(&seen), None);
        seen.insert(7, 3);
        assert_eq!(evict_oldest_seen(&seen), Some(7));
    }

    #[test]
    fn reasons_round_trip_through_their_codes() {
        for reason in [OfflineReason::NoSignal, OfflineReason::Removed] {
            assert_eq!(OfflineReason::from_code(reason.code()), Some(reason));
        }
        assert_eq!(OfflineReason::from_code("other"), None);
    }
}
