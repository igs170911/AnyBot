use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomUnavailable {
    pub reason: String,
}

pub trait RandomSource: Send + Sync {
    fn next_u64(&self) -> Result<u64, RandomUnavailable>;
}

#[derive(Debug, Default)]
pub struct OsRandom;

impl RandomSource for OsRandom {
    fn next_u64(&self) -> Result<u64, RandomUnavailable> {
        getrandom::u64().map_err(|e| RandomUnavailable {
            reason: e.to_string(),
        })
    }
}

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug)]
pub struct SeededRandom {
    state: AtomicU64,
}

impl SeededRandom {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        SeededRandom {
            state: AtomicU64::new(seed),
        }
    }
}

impl RandomSource for SeededRandom {
    fn next_u64(&self) -> Result<u64, RandomUnavailable> {
        let step = self
            .state
            .fetch_add(GAMMA, Ordering::SeqCst)
            .wrapping_add(GAMMA);
        let first = (step ^ (step >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let second = (first ^ (first >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Ok(second ^ (second >> 31))
    }
}

pub const JITTER_MIN_MS: u64 = 500;
pub const JITTER_MAX_MS: u64 = 2_000;

pub fn jitter_ms(random: &dyn RandomSource) -> Result<u64, RandomUnavailable> {
    let span = JITTER_MAX_MS
        .saturating_sub(JITTER_MIN_MS)
        .saturating_add(1);
    let pick = random
        .next_u64()?
        .checked_rem(span)
        .ok_or(RandomUnavailable {
            reason: "jitter span is zero".to_owned(),
        })?;
    Ok(JITTER_MIN_MS.saturating_add(pick))
}

#[cfg(test)]
#[path = "reconnect_random_tests.rs"]
mod tests;
