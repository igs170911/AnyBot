pub const K114_1_RECENT_STEPS: usize = 10;
pub const K114_2_DEFAULT_RESULT_CHARS: usize = 8_000;
pub const K114_2_ERROR_WINDOW_THIRDS: usize = 3;
pub const K114_3_SHORT_RESULT_CHARS: usize = 600;
pub const K114_3_ACCUMULATE_MARGIN_MILLI: u64 = 150;
pub const K114_4_WATERMARK_MILLI: u64 = 700;
pub const K114_4_MIN_VERBATIM_STEPS: usize = 2;
pub const K114_5_USER_REF_MILLI: u64 = 100;
pub const K114_5_REF_HEAD_CHARS: usize = 1_000;
pub const K114_6_MEMORY_CAP_MILLI: u64 = 200;
pub const K114_6_MEMORY_MIN_KEEP: usize = 3;
pub const K114_7_STATE_MAX_TOKENS: u64 = 2_000;

#[must_use]
pub fn milli_of(value: u64, milli: u64) -> u64 {
    value.saturating_mul(milli).saturating_add(999) / 1_000
}
