use anybot_foundation::Secret;

use super::*;

#[test]
fn generated_key_has_uncompressed_public_key() {
    let key = VapidKey::generate().unwrap();
    let public = key.public_b64().unwrap();
    assert_eq!(public.len(), 87);
    assert!(public.starts_with('B'));
}

#[test]
fn scalar_round_trips_to_same_public_key() {
    let key = VapidKey::generate().unwrap();
    let again =
        VapidKey::from_scalar(Secret::from_untrusted(key.scalar().expose().to_owned())).unwrap();
    assert_eq!(key.public_b64().unwrap(), again.public_b64().unwrap());
}

#[test]
fn malformed_scalar_is_refused() {
    let short = Secret::from_untrusted("abc".to_owned());
    assert!(matches!(
        VapidKey::from_scalar(short),
        Err(PushError::VapidKey)
    ));
}

#[test]
fn debug_output_hides_scalar() {
    let key = VapidKey::generate().unwrap();
    let shown = format!("{key:?}");
    assert!(!shown.contains(key.scalar().expose()));
}
