pub const RETRY_CAP_SECONDS: u64 = 30;

#[must_use]
pub fn retry_delay_secs(attempt: u64) -> u64 {
    let doubled = 1u64 << attempt.min(20);
    doubled.min(RETRY_CAP_SECONDS)
}

#[must_use]
pub fn retry_schedule(count: u64) -> Vec<u64> {
    (0..count).map(retry_delay_secs).collect()
}
