pub trait RandomSource: Send + Sync {
    fn next_u64(&self) -> u64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRandom;

impl RandomSource for SystemRandom {
    fn next_u64(&self) -> u64 {
        rand::random()
    }
}

#[must_use]
pub fn span_ms(random: &dyn RandomSource, low_ms: u64, high_ms: u64) -> u64 {
    let width = high_ms.saturating_sub(low_ms).saturating_add(1);
    random
        .next_u64()
        .checked_rem(width)
        .map_or(low_ms, |offset| low_ms.saturating_add(offset))
}
