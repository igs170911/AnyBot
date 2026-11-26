use std::time::Duration;

use crate::types::Audience;

pub const WATCHING_TRIES: u32 = 3;
pub const UNATTENDED_TRIES: u32 = 8;
pub const BUSY_WAIT: Duration = Duration::from_secs(8);
pub const RETRY_AFTER_CAP: Duration = Duration::from_secs(30);
pub const WATCHING_CAP: Duration = Duration::from_secs(4);
pub const UNATTENDED_CAP: Duration = Duration::from_secs(30);
pub const BLANK_EXTRA: u32 = 1;

#[must_use]
pub fn try_limit(audience: Audience) -> u32 {
    match audience {
        Audience::Watching => WATCHING_TRIES,
        Audience::Unattended => UNATTENDED_TRIES,
    }
}

#[must_use]
pub fn base_wait(audience: Audience, attempt: u32) -> Duration {
    match audience {
        Audience::Watching => {
            let seconds = 1_u64
                .saturating_mul(1_u64.checked_shl(attempt).unwrap_or(4))
                .min(4);
            Duration::from_secs(seconds)
        }
        Audience::Unattended => {
            let seconds = 1_u64
                .saturating_mul(1_u64.checked_shl(attempt).unwrap_or(30))
                .min(30);
            Duration::from_secs(seconds)
        }
    }
}

#[must_use]
pub fn apply_jitter(base: Duration, jitter_milli: i32, audience: Audience) -> Duration {
    if !matches!(audience, Audience::Unattended) {
        return base.min(WATCHING_CAP);
    }
    let millis = u64_millis(base);
    let delta = i128::from(millis).saturating_mul(i128::from(jitter_milli)) / 1_000;
    let adjusted = i128::from(millis).saturating_add(delta).max(0);
    let capped = u64::try_from(adjusted).unwrap_or(u64::MAX);
    Duration::from_millis(capped).min(UNATTENDED_CAP)
}

#[must_use]
pub fn retry_after(header_secs: Option<u64>, fallback: Duration) -> Duration {
    match header_secs {
        Some(secs) => Duration::from_secs(secs).min(RETRY_AFTER_CAP),
        None => fallback,
    }
}

#[must_use]
pub fn retry_spans(audience: Audience, jitter_milli: i32) -> Vec<Duration> {
    (0..try_limit(audience).saturating_sub(1))
        .map(|attempt| apply_jitter(base_wait(audience, attempt), jitter_milli, audience))
        .collect()
}

fn u64_millis(span: Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}
