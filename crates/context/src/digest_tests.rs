use super::content_digest;

#[test]
fn test_digest_is_stable_and_distinguishes_content() {
    assert_eq!(content_digest("abc"), content_digest("abc"));
    assert_ne!(content_digest("abc"), content_digest("abd"));
    assert_eq!(content_digest("").len(), 16);
}
