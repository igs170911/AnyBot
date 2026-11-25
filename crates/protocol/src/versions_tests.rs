use jiff::civil::{Date, Time};
use jiff::tz::Offset;

use anybot_foundation::{Clock, SystemClock, TimestampMs};

use super::{ProtocolRelease, accepts_protocol_version, accepts_with};

struct FixedClock(TimestampMs);

impl Clock for FixedClock {
    fn now_ms(&self) -> TimestampMs {
        self.0
    }
}

fn release(version: u32, year: i16, month: i8, day: i8) -> ProtocolRelease {
    ProtocolRelease::new(version, Date::new(year, month, day).unwrap())
}

fn ms_at(year: i16, month: i8, day: i8, hour: i8) -> TimestampMs {
    let date = Date::new(year, month, day).unwrap();
    let dt = date.to_datetime(Time::new(hour, 0, 0, 0).unwrap());
    let timestamp = Offset::UTC.to_timestamp(dt).unwrap();
    TimestampMs::from_millis(u64::try_from(timestamp.as_millisecond()).unwrap())
}

#[test]
fn test_versions_real_table_accepts_only_v1_with_system_clock() {
    assert!(accepts_protocol_version(1, &SystemClock));
    assert!(!accepts_protocol_version(2, &SystemClock));
    assert!(!accepts_protocol_version(0, &SystemClock));
}

#[test]
fn test_versions_accepts_latest_and_rejects_higher() {
    let releases = [release(2, 2026, 1, 1)];
    let clock = FixedClock(TimestampMs::from_millis(1_700_000_000_000));
    assert!(accepts_with(&releases, 2, &clock));
    assert!(!accepts_with(&releases, 3, &clock));
    assert!(!accepts_with(&releases, 0, &clock));
}

#[test]
fn test_versions_empty_table_rejects_everything() {
    let clock = FixedClock(TimestampMs::from_millis(1_700_000_000_000));
    assert!(!accepts_with(&[], 1, &clock));
    assert!(!accepts_with(&[], 2, &clock));
}

#[test]
fn test_versions_superseded_is_accepted_until_cutoff_day() {
    let releases = [release(1, 2025, 6, 1), release(2, 2026, 6, 15)];
    assert!(accepts_with(
        &releases,
        1,
        &FixedClock(ms_at(2026, 12, 15, 0))
    ));
    assert!(!accepts_with(
        &releases,
        1,
        &FixedClock(ms_at(2026, 12, 16, 0))
    ));
}

#[test]
fn test_versions_june_release_cutoff_is_same_year_december() {
    let cutoff = release(2, 2026, 6, 15).cutoff().unwrap();
    assert_eq!(cutoff, Date::new(2026, 12, 15).unwrap());
}

#[test]
fn test_versions_cutoff_clamps_end_of_month_for_every_release_month() {
    let expected: [(i16, i8, i8); 12] = [
        (2026, 7, 31),
        (2026, 8, 28),
        (2026, 9, 30),
        (2026, 10, 30),
        (2026, 11, 30),
        (2026, 12, 30),
        (2027, 1, 31),
        (2027, 2, 28),
        (2027, 3, 30),
        (2027, 4, 30),
        (2027, 5, 30),
        (2027, 6, 30),
    ];
    let days: [i8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (index, (want, last_day)) in expected.iter().copied().zip(days).enumerate() {
        let month = u32::try_from(index).unwrap() + 1;
        let cutoff = release(2, 2026, i8::try_from(month).unwrap(), last_day)
            .cutoff()
            .unwrap();
        let (year, exp_month, day) = want;
        assert_eq!(
            cutoff,
            Date::new(year, exp_month, day).unwrap(),
            "月份 {month}"
        );
    }
}

#[test]
fn test_versions_august_cutoff_crosses_year() {
    let cutoff = release(2, 2026, 8, 31).cutoff().unwrap();
    assert_eq!(cutoff, Date::new(2027, 2, 28).unwrap());
    let releases = [release(1, 2025, 6, 1), release(2, 2026, 8, 31)];
    assert!(accepts_with(
        &releases,
        1,
        &FixedClock(ms_at(2027, 2, 28, 23))
    ));
    assert!(!accepts_with(
        &releases,
        1,
        &FixedClock(ms_at(2027, 3, 1, 0))
    ));
}

#[test]
fn test_versions_clock_far_beyond_cutoff_rejects_superseded() {
    let releases = [release(1, 2025, 6, 1), release(2, 2026, 8, 31)];
    let far = FixedClock(ms_at(2080, 1, 1, 0));
    assert!(!accepts_with(&releases, 1, &far));
    assert!(accepts_with(&releases, 2, &far));
}

#[test]
fn test_versions_unrepresentable_clock_rejects_superseded() {
    let releases = [release(1, 2025, 6, 1), release(2, 2026, 8, 31)];
    assert!(!accepts_with(
        &releases,
        1,
        &FixedClock(TimestampMs::from_millis(u64::MAX))
    ));
    assert!(accepts_with(
        &releases,
        2,
        &FixedClock(TimestampMs::from_millis(u64::MAX))
    ));
}

#[test]
fn test_versions_today_from_ms_matches_jiff_dates() {
    assert_eq!(ms_at(1970, 1, 1, 0).millis(), 0);
    let today = super::today_from(ms_at(2026, 6, 15, 12)).unwrap();
    assert_eq!(today, Date::new(2026, 6, 15).unwrap());
    let future = super::today_from(ms_at(9999, 12, 30, 21)).unwrap();
    assert_eq!(future, Date::new(9999, 12, 30).unwrap());
    assert!(super::today_from(TimestampMs::from_millis(u64::MAX)).is_err());
}

#[test]
fn test_versions_rejects_negative_clock_as_epoch() {
    let releases = [release(1, 2025, 6, 1), release(2, 2026, 8, 31)];
    assert!(accepts_with(
        &releases,
        1,
        &FixedClock(TimestampMs::from_millis(0))
    ));
}
