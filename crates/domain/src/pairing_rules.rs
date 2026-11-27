pub const CODE_DIGITS: usize = 6;
pub const CODE_TTL_MS: u64 = 600_000;
pub const PENDING_CAP: i64 = 10_000;
pub const PASSWORD_FRESH_MS: u64 = 900_000;
pub const ADDRESS_FAIL_LIMIT: i64 = 8;
pub const GLOBAL_FAIL_LIMIT: i64 = 200;
pub const WRONG_DELAY_MS: u64 = 300;
pub const CODE_SPACE: u32 = 1_000_000;
const UNBIASED_LIMIT: u32 = u32::MAX - (u32::MAX % CODE_SPACE);

#[must_use]
pub fn code_from_bytes(bytes: [u8; 4]) -> Option<String> {
    let raw = u32::from_be_bytes(bytes);
    if raw >= UNBIASED_LIMIT {
        return None;
    }
    Some(format!("{:06}", raw % CODE_SPACE))
}

#[must_use]
pub fn code_has_valid_shape(text: &str) -> bool {
    text.len() == CODE_DIGITS && text.bytes().all(|byte| byte.is_ascii_digit())
}

#[must_use]
pub fn password_is_fresh(entered_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(entered_ms) <= PASSWORD_FRESH_MS
}

#[must_use]
pub fn code_is_alive(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms < expires_at_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits_with_leading_zeros_kept() {
        assert_eq!(code_from_bytes([0, 0, 0, 7]).as_deref(), Some("000007"));
        assert_eq!(code_from_bytes([0, 15, 66, 63]).as_deref(), Some("999999"));
    }

    #[test]
    fn the_biased_tail_of_the_range_is_refused() {
        assert_eq!(code_from_bytes([255, 255, 255, 255]), None);
    }

    #[test]
    fn shape_needs_exactly_six_ascii_digits() {
        assert!(code_has_valid_shape("048213"));
        assert!(!code_has_valid_shape("04821"));
        assert!(!code_has_valid_shape("0482133"));
        assert!(!code_has_valid_shape("04821x"));
        assert!(!code_has_valid_shape(
            "\u{0664}\u{0668}\u{0662}\u{0661}\u{0663}\u{0660}"
        ));
    }

    #[test]
    fn a_password_is_fresh_for_fifteen_minutes() {
        assert!(password_is_fresh(1_000, 1_000 + PASSWORD_FRESH_MS));
        assert!(!password_is_fresh(1_000, 1_001 + PASSWORD_FRESH_MS));
    }

    #[test]
    fn a_code_dies_at_its_expiry_instant() {
        assert!(code_is_alive(10, 9));
        assert!(!code_is_alive(10, 10));
    }
}
