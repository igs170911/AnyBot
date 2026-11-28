use crate::secret_mask::mask_secrets;

const DOTS: &str = "\u{2022}\u{2022}\u{2022}\u{2022}";
const PEM_SAMPLE: &str = concat!(
    "-----BEGIN ",
    "PRIVATE KEY-----\nMIIEvQIBADANBg\n-----END PRIVATE KEY-----",
);

#[test]
fn mask_secrets_rules() {
    let cases = [
        ("glpat-abc123456", format!("glpat-{DOTS}"), "abc123456"),
        ("ghp_abcdef123456", format!("ghp_{DOTS}"), "abcdef123456"),
        (
            "github_pat_11AAbbCC",
            format!("github_pat_{DOTS}"),
            "11AAbbCC",
        ),
        ("sk-proj123456", format!("sk-{DOTS}"), "proj123456"),
        ("xai-key998877", format!("xai-{DOTS}"), "key998877"),
        ("AKIAIOSFODNN7EXAMPLE", format!("AKIA{DOTS}"), "IOSFODNN7"),
        ("ASIAIOSFODNN7EXAMPLE", format!("ASIA{DOTS}"), "IOSFODNN7"),
        ("xoxb-12345-67890", format!("xox{DOTS}"), "12345"),
        ("AIzaSyD-998877", format!("AIza{DOTS}"), "998877"),
        (
            "Authorization: Bearer tokSecret99",
            format!("Authorization: Bearer {DOTS}"),
            "tokSecret99",
        ),
        (
            "Authorization: Basic dXNlcjpwYXNz",
            format!("Authorization: Basic {DOTS}"),
            "dXNlcjpwYXNz",
        ),
        (
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.sigXYZ",
            format!("eyJ{DOTS}"),
            "sigXYZ",
        ),
        ("API_KEY=xyz789", format!("API_KEY={DOTS}"), "xyz789"),
        (
            "\"password\": \"mySecretPass\"",
            format!("\"password\": \"{DOTS}\""),
            "mySecretPass",
        ),
        (
            "MY_TOKEN=ghp_keepPrefix99",
            format!("MY_TOKEN=ghp_{DOTS}"),
            "keepPrefix99",
        ),
        (
            PEM_SAMPLE,
            format!(
                concat!(
                    "-----BEGIN ",
                    "PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----"
                ),
                DOTS
            ),
            "MIIEvQIBADANBg",
        ),
    ];
    for (input, expected, hidden) in cases {
        let masked = mask_secrets(input);
        assert_eq!(masked, expected);
        assert!(!masked.contains(hidden));
    }
    let plain = "git status && cargo check --disk-cache";
    assert_eq!(mask_secrets(plain), plain);
}
