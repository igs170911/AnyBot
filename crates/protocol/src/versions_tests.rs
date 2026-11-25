use super::*;
use anybot_foundation::Clock;

struct FixedClock(u64);

impl Clock for FixedClock {
    fn now_ms(&self) -> TimestampMs {
        TimestampMs::from_millis(self.0)
    }
}

const DAY: u64 = 86_400_000;

fn ms_at(y: u64, mo: u64, d: u64) -> u64 {
    let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1)
        .ok()
        .unwrap_or(time::Date::MIN);
    let year = i32::try_from(y).unwrap_or(0);
    let month =
        time::Month::try_from(u8::try_from(mo).unwrap_or(1)).unwrap_or(time::Month::January);
    let day = u8::try_from(d).unwrap_or(1);
    let dt = time::Date::from_calendar_date(year, month, day)
        .ok()
        .unwrap_or(epoch)
        .midnight()
        .assume_utc();
    u64::try_from(dt.unix_timestamp())
        .unwrap_or(0)
        .checked_mul(1_000)
        .unwrap_or(0)
}

fn release(version: u32, y: u32, mo: u32, d: u32) -> ProtocolRelease {
    ProtocolRelease {
        version,
        released_year: y,
        released_month: mo,
        released_day: d,
    }
}

const V2: &[ProtocolRelease] = &[
    ProtocolRelease {
        version: 1,
        released_year: 2026,
        released_month: 1,
        released_day: 1,
    },
    ProtocolRelease {
        version: 2,
        released_year: 2026,
        released_month: 8,
        released_day: 31,
    },
];

#[test]
fn test_versions_latest_is_always_accepted() {
    let far_future = FixedClock(ms_at(2126, 1, 1));
    assert!(accepts_protocol_version(PROTOCOL_VERSION, &far_future));
}

#[test]
fn test_versions_newer_than_latest_is_rejected() {
    let clock = FixedClock(ms_at(2026, 2, 1));
    assert!(!accepts_protocol_version(2, &clock));
    assert!(!accepts_protocol_version(99, &clock));
    assert!(!accepts_protocol_version(0, &clock));
}

#[test]
fn test_versions_superseded_boundaries_at_cutoff_day() {
    let cutoff_ms = ms_at(2027, 2, 28);
    let one_before = FixedClock(cutoff_ms.saturating_sub(DAY));
    let exact = FixedClock(cutoff_ms);
    let one_after = FixedClock(cutoff_ms.saturating_add(DAY));
    assert!(accepts_with(V2, 1, &one_before));
    assert!(accepts_with(V2, 1, &exact));
    assert!(!accepts_with(V2, 1, &one_after));
    assert!(accepts_with(V2, 2, &one_after));
}

#[test]
fn test_versions_clamps_missing_days_to_last_day_of_month() {
    assert_eq!(cutoff_date(&release(2, 2026, 8, 31)), (2027, 2, 28));
    assert_eq!(cutoff_date(&release(2, 2025, 8, 31)), (2026, 2, 28));
    assert_eq!(cutoff_date(&release(2, 2026, 9, 30)), (2027, 3, 30));
    assert_eq!(cutoff_date(&release(2, 2026, 10, 31)), (2027, 4, 30));
    assert_eq!(cutoff_date(&release(2, 2026, 1, 31)), (2026, 7, 31));
    assert_eq!(cutoff_date(&release(2, 2026, 12, 31)), (2027, 6, 30));
    assert_eq!(cutoff_date(&release(2, 2026, 8, 30)), (2027, 2, 28));
}

#[test]
fn test_versions_leap_february_allows_29() {
    assert_eq!(cutoff_date(&release(2, 2027, 8, 31)), (2028, 2, 29));
}

#[test]
fn test_versions_today_from_ms_matches_expected_dates() {
    assert_eq!(
        today_from(TimestampMs::from_millis(ms_at(2026, 1, 1))),
        (2026, 1, 1)
    );
    assert_eq!(
        today_from(TimestampMs::from_millis(ms_at(2027, 2, 28))),
        (2027, 2, 28)
    );
    assert_eq!(today_from(TimestampMs::from_millis(0)), (1970, 1, 1));
    assert_eq!(
        today_from(TimestampMs::from_millis(ms_at(2000, 3, 1))),
        (2000, 3, 1)
    );
}
