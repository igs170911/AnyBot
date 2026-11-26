pub const ACCOUNT_MIN_LEN: usize = 2;
pub const ACCOUNT_MAX_LEN: usize = 64;
pub const PASSWORD_MIN_LEN: usize = 8;

#[must_use]
pub fn is_allowed_account_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '@' | '-')
}

pub fn normalize_account_name(raw: &str) -> Result<String, String> {
    let lowered = raw.to_ascii_lowercase();
    if lowered.len() < ACCOUNT_MIN_LEN || lowered.len() > ACCOUNT_MAX_LEN {
        return Err("account length out of range".to_owned());
    }
    if !lowered.chars().all(is_allowed_account_char) {
        return Err("account characters not allowed".to_owned());
    }
    Ok(lowered)
}

#[must_use]
pub fn is_allowed_password(raw: &str) -> bool {
    raw.chars().count() >= PASSWORD_MIN_LEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_rule_rejects_short_long_spaced_and_symbol() {
        assert_eq!(
            normalize_account_name("a"),
            Err("account length out of range".to_owned())
        );
        assert!(normalize_account_name("bob has space").is_err());
        assert!(normalize_account_name("bob#1").is_err());
        assert!(normalize_account_name(&"x".repeat(65)).is_err());
        assert_eq!(normalize_account_name("Bob").ok(), Some("bob".to_owned()));
        assert_eq!(
            normalize_account_name("a.b_c@d-e").ok(),
            Some("a.b_c@d-e".to_owned())
        );
    }

    #[test]
    fn password_rule_counts_characters() {
        assert!(!is_allowed_password("1234567"));
        assert!(is_allowed_password("12345678"));
        let seven_cjk = "\u{5BC6}\u{78BC}\u{4E03}\u{500B}\u{5B57}\u{52A0}\u{4E00}";
        let eight_cjk = "\u{5BC6}\u{78BC}\u{7801}\u{4E03}\u{500B}\u{5B57}\u{52A0}\u{4E00}";
        assert!(!is_allowed_password(seven_cjk));
        assert!(is_allowed_password(eight_cjk));
    }
}
