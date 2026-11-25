use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use anybot_foundation::{Clock, TimestampMs};

pub struct ProtocolRelease {
    pub version: u32,
    pub released: Date,
}

impl ProtocolRelease {
    #[must_use]
    pub fn new(version: u32, released: Date) -> Self {
        ProtocolRelease { version, released }
    }

    pub fn cutoff(&self) -> Result<Date, jiff::Error> {
        self.released.checked_add(grace_months())
    }
}

pub const PROTOCOL_RELEASES: [ProtocolRelease; 1] = [ProtocolRelease {
    version: crate::events::PROTOCOL_VERSION,
    released: jiff::civil::date(2026, 1, 1),
}];

fn grace_months() -> Span {
    Span::new().months(6)
}

#[must_use]
pub fn accepts_protocol_version(version: u32, clock: &dyn Clock) -> bool {
    accepts_with(&PROTOCOL_RELEASES, version, clock)
}

#[must_use]
pub fn accepts_with(releases: &[ProtocolRelease], version: u32, clock: &dyn Clock) -> bool {
    let Some(latest) = releases.iter().map(|release| release.version).max() else {
        return false;
    };
    if version == latest {
        return true;
    }
    if version > latest || version == 0 {
        return false;
    }
    let Some(superseded_by) = releases
        .iter()
        .filter(|release| release.version > version)
        .min_by_key(|release| release.version)
    else {
        return false;
    };
    let Ok(cutoff) = superseded_by.cutoff() else {
        return false;
    };
    let Ok(today) = today_from(clock.now_ms()) else {
        return false;
    };
    today <= cutoff
}

fn today_from(ms: TimestampMs) -> Result<Date, jiff::Error> {
    let millis = i64::try_from(ms.millis())
        .map_err(|_| jiff::Error::from_args(format_args!("毫秒超過 Timestamp 可表示的範圍")))?;
    let timestamp = Timestamp::from_millisecond(millis)?;
    Ok(timestamp.to_zoned_datetime(TimeZone::UTC).datetime().date())
}

#[cfg(test)]
#[path = "versions_tests.rs"]
mod tests;
