use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(transparent)]
pub struct TimestampMs(u64);

impl TimestampMs {
    #[must_use]
    pub fn from_millis(millis: u64) -> Self {
        TimestampMs(millis)
    }

    #[must_use]
    pub fn millis(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn rfc3339(self) -> Option<String> {
        let secs = i64::try_from(self.0 / 1_000).ok()?;
        let micros = u32::try_from(self.0 % 1_000).ok()?.saturating_mul(1_000);
        let dt = OffsetDateTime::from_unix_timestamp(secs).ok()?;
        let dt = dt.replace_microsecond(micros).ok()?;
        Some(
            dt.format(&time::format_description::well_known::Rfc3339)
                .ok()?
                .clone(),
        )
    }
}

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> TimestampMs;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> TimestampMs {
        let now = OffsetDateTime::now_utc();
        let secs = u64::try_from(now.unix_timestamp()).unwrap_or(0);
        let sub_ms = u64::from(now.nanosecond() / 1_000_000);
        TimestampMs(secs.saturating_mul(1_000).saturating_add(sub_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedClock(u64);

    impl Clock for FixedClock {
        fn now_ms(&self) -> TimestampMs {
            TimestampMs(self.0)
        }
    }

    #[test]
    fn test_clock_fixed_ms_and_system_ms_are_close() {
        let fixed = FixedClock(1_700_000_000_000);
        assert_eq!(fixed.now_ms().millis(), 1_700_000_000_000);
        let sys = SystemClock.now_ms().millis();
        assert!(sys > 1_700_000_000_000);
    }

    #[test]
    fn test_clock_rfc3339_rounds_and_rejects_huge() {
        let ts = TimestampMs(1_700_000_000_500);
        let text = ts.rfc3339();
        assert!(text.is_some_and(|s| s.starts_with("2023-11-14T22:13:20.5")));
        assert_eq!(TimestampMs(u64::MAX).rfc3339(), None);
    }

    #[test]
    fn test_clock_ordering_is_monotonic_by_millis() {
        assert!(TimestampMs(2) > TimestampMs(1));
        assert!(TimestampMs(1) < TimestampMs(2));
    }
}
