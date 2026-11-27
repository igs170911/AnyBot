use super::*;

const RFC_SECRET: &[u8] = b"12345678901234567890";

#[test]
fn totp_matches_the_rfc_6238_sha1_vectors_in_six_digits() {
    let vectors: [(i64, &str); 6] = [
        (59_000, "287082"),
        (1_111_111_109_000, "081804"),
        (1_111_111_111_000, "050471"),
        (1_234_567_890_000, "005924"),
        (2_000_000_000_000, "279037"),
        (20_000_000_000_000, "353130"),
    ];
    for (millis, want) in vectors {
        let step = time_step(millis).unwrap();
        assert_eq!(totp_code(RFC_SECRET, step).unwrap(), want, "at {millis}ms");
    }
}

#[test]
fn window_accepts_the_previous_and_next_step_only() {
    let now_ms = 1_700_000_015_000;
    let now = time_step(now_ms).unwrap();
    for (offset, accepted) in [(-2, false), (-1, true), (0, true), (1, true), (2, false)] {
        let step = now.checked_add(offset).unwrap();
        let code = totp_code(RFC_SECRET, step).unwrap();
        let found = matching_step(RFC_SECRET, now_ms, &code).unwrap();
        assert_eq!(found.is_some(), accepted, "offset {offset}");
        if accepted {
            assert_eq!(found, Some(step));
        }
    }
}

#[test]
fn codes_are_classified_by_shape() {
    assert_eq!(
        classify_code(" 123456 "),
        CodeInput::Totp("123456".to_owned())
    );
    assert_eq!(
        classify_code("0A1b2C3d4E"),
        CodeInput::Recovery("0a1b2c3d4e".to_owned())
    );
    for bad in ["12345", "1234567", "12345a", "0a1b2c3d4", "zzzzzzzzzz", ""] {
        assert_eq!(classify_code(bad), CodeInput::Malformed, "{bad}");
    }
}

#[test]
fn challenge_closes_after_five_minutes_or_three_tries() {
    let created = 1_000_000;
    assert!(challenge_open(created, 0, 1_299_999));
    assert!(!challenge_open(created, 0, 1_300_000));
    assert!(challenge_open(created, 2, created));
    assert!(!challenge_open(created, 3, created));
}

#[test]
fn reuse_floor_is_two_minutes_back() {
    assert_eq!(reuse_floor(500_000), 380_000);
}

#[test]
fn recovery_codes_are_ten_lowercase_hex_chars() {
    let code = recovery_code(&[0xab, 0x01, 0xff, 0x10, 0x9c]);
    assert_eq!(code, "ab01ff109c");
}

#[test]
fn otpauth_uri_carries_issuer_period_digits_and_base32_secret() {
    let uri = otpauth_uri("alice@home", RFC_SECRET);
    assert_eq!(
        uri,
        "otpauth://totp/AnyBot:alice%40home?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=AnyBot&period=30&digits=6"
    );
}

#[test]
fn text_comparison_needs_equal_length_and_bytes() {
    assert!(same_text("123456", "123456"));
    assert!(!same_text("123456", "123457"));
    assert!(!same_text("123456", "12345"));
}
