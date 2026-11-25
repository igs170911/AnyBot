use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{sleep, sleep_until};

pub type PinBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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

    fn sleep(&self, span: Duration) -> PinBoxFuture<'static, ()>;

    fn sleep_until(&self, deadline: TimestampMs) -> PinBoxFuture<'static, ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockExpired;

pub fn timeout<T>(
    clock: &dyn Clock,
    span: Duration,
    task: T,
) -> PinBoxFuture<'static, Result<T::Output, ClockExpired>>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let deadline = clock.now_ms().saturating_add_ms(span);
    let wake = clock.sleep_until(deadline);
    Box::pin(async move {
        tokio::pin!(task);
        tokio::select! {
            biased;
            output = &mut task => Ok(output),
            _elapsed = wake => Err(ClockExpired),
        }
    })
}

impl TimestampMs {
    #[must_use]
    pub fn saturating_add_ms(self, span: Duration) -> Self {
        let add_ms = u64::try_from(span.as_millis()).unwrap_or(u64::MAX);
        TimestampMs(self.0.saturating_add(add_ms))
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> TimestampMs {
        let now = OffsetDateTime::now_utc();
        system_now_ms(
            now.unix_timestamp(),
            u64::from(now.nanosecond()) / 1_000_000,
        )
    }

    fn sleep(&self, span: Duration) -> PinBoxFuture<'static, ()> {
        Box::pin(sleep(span))
    }

    fn sleep_until(&self, deadline: TimestampMs) -> PinBoxFuture<'static, ()> {
        Box::pin(sleep_until(tokio_deadline(deadline, self.now_ms())))
    }
}

fn tokio_deadline(deadline: TimestampMs, now: TimestampMs) -> tokio::time::Instant {
    let span_ms = deadline.millis().saturating_sub(now.millis());
    tokio::time::Instant::now()
        .checked_add(Duration::from_millis(span_ms))
        .unwrap_or_else(tokio::time::Instant::now)
}

fn system_now_ms(secs: i64, sub_ms: u64) -> TimestampMs {
    let Some(secs_ms) = u64::try_from(secs)
        .ok()
        .and_then(|positive| positive.checked_mul(1_000))
    else {
        return TimestampMs::from_millis(sub_ms);
    };
    TimestampMs::from_millis(secs_ms.saturating_add(sub_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedClock(u64);

    impl Clock for FixedClock {
        fn now_ms(&self) -> TimestampMs {
            TimestampMs(self.0)
        }

        fn sleep(&self, _span: Duration) -> PinBoxFuture<'static, ()> {
            Box::pin(std::future::ready(()))
        }

        fn sleep_until(&self, _deadline: TimestampMs) -> PinBoxFuture<'static, ()> {
            Box::pin(std::future::ready(()))
        }
    }

    #[tokio::test]
    async fn test_clock_system_sleep_and_timeout_complete() {
        let clock = SystemClock;
        clock.sleep(Duration::from_millis(10)).await;
        let outcome = timeout(&clock, Duration::from_secs(5), std::future::ready(7u8)).await;
        assert_eq!(outcome, Ok(7u8));
    }

    #[tokio::test]
    async fn test_clock_system_timeout_reports_expiry() {
        let clock = SystemClock;
        let outcome = timeout(
            &clock,
            Duration::from_millis(1),
            clock.sleep(Duration::from_secs(30)),
        )
        .await;
        assert_eq!(outcome, Err(ClockExpired));
    }

    #[tokio::test]
    async fn test_clock_fixed_sleep_resolves_immediately() {
        let fixed = FixedClock(1_700_000_000_000);
        fixed.sleep(Duration::from_secs(9)).await;
        fixed.sleep_until(TimestampMs::from_millis(9)).await;
        let outcome = timeout(&fixed, Duration::from_secs(1), std::future::ready(3u8)).await;
        assert_eq!(outcome, Ok(3u8));
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
