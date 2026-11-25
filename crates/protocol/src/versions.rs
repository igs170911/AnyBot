use anybot_foundation::{Clock, TimestampMs};

use crate::events::PROTOCOL_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolRelease {
    pub version: u32,
    pub released_year: u32,
    pub released_month: u32,
    pub released_day: u32,
}

pub const PROTOCOL_RELEASES: [ProtocolRelease; 1] = [ProtocolRelease {
    version: 1,
    released_year: 2026,
    released_month: 1,
    released_day: 1,
}];

const GRACE_MONTHS: u32 = 6;

#[must_use]
pub fn latest_protocol_version() -> u32 {
    PROTOCOL_RELEASES
        .iter()
        .map(|r| r.version)
        .max()
        .unwrap_or(PROTOCOL_VERSION)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    if month == 2 {
        let leap = year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400);
        return if leap { 29 } else { 28 };
    }
    if matches!(month, 4 | 6 | 9 | 11) {
        30
    } else {
        31
    }
}

fn cutoff_date(release: &ProtocolRelease) -> (u64, u64, u64) {
    let total_month = u64::from(release.released_month)
        .checked_add(u64::from(GRACE_MONTHS))
        .unwrap_or(0);
    let year = u64::from(release.released_year)
        .checked_add(total_month.div_euclid(12))
        .unwrap_or(0);
    let month = total_month.rem_euclid(12);
    let month = if month == 0 { 12 } else { month };
    let day = u64::from(release.released_day).min(u64::from(days_in_month(
        u32::try_from(year).unwrap_or(0),
        u32::try_from(month).unwrap_or(0),
    )));
    (year, month, day)
}

fn today_from(ms: TimestampMs) -> (u64, u64, u64) {
    let half = ms.millis().checked_div(1_000).unwrap_or(u64::MAX);
    let secs = i64::try_from(half).unwrap_or(i64::MAX);
    let dt =
        time::OffsetDateTime::from_unix_timestamp(secs).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let (year, month, day) = dt.to_calendar_date();
    let month: u8 = month.into();
    (
        u64::try_from(year).unwrap_or(0),
        u64::from(month),
        u64::from(day),
    )
}

fn is_at_or_before(today: (u64, u64, u64), cutoff: (u64, u64, u64)) -> bool {
    today.0 < cutoff.0
        || (today.0 == cutoff.0
            && (today.1 < cutoff.1 || (today.1 == cutoff.1 && today.2 <= cutoff.2)))
}

#[must_use]
pub fn accepts_with(releases: &[ProtocolRelease], version: u32, clock: &dyn Clock) -> bool {
    let latest = releases.iter().map(|r| r.version).max().unwrap_or(0);
    if version == latest {
        return true;
    }
    if version > latest || version == 0 {
        return false;
    }
    let Some(superseded_by) = releases
        .iter()
        .filter(|r| r.version > version)
        .min_by_key(|r| r.version)
    else {
        return false;
    };
    is_at_or_before(today_from(clock.now_ms()), cutoff_date(superseded_by))
}

#[must_use]
pub fn accepts_protocol_version(version: u32, clock: &dyn Clock) -> bool {
    accepts_with(&PROTOCOL_RELEASES, version, clock)
}

#[cfg(test)]
#[path = "versions_tests.rs"]
mod tests;
