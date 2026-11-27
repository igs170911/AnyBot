use anybot_foundation::{MasterKey, Secret, SecretBytes};

use super::SecretPlace;
use crate::secret_keys::{SecretError, SecretKeys};

const HOME: SecretPlace<'static> = SecretPlace {
    table: "two_step",
    column: "secret_sealed",
    row: "alice",
};

fn keys() -> SecretKeys {
    SecretKeys::from_master(&MasterKey::generate().unwrap()).unwrap()
}

fn plain() -> SecretBytes {
    SecretBytes::from_untrusted(b"shared secret bytes".to_vec())
}

#[test]
fn sealed_secret_opens_at_its_own_place() {
    let keys = keys();
    let sealed = keys.seal(&HOME, &plain()).unwrap();
    assert_eq!(
        keys.open(&HOME, &sealed).unwrap().expose(),
        plain().expose()
    );
}

#[test]
fn moved_row_or_column_or_table_does_not_open() {
    let keys = keys();
    let sealed = keys.seal(&HOME, &plain()).unwrap();
    let moved_row = SecretPlace { row: "bob", ..HOME };
    let moved_column = SecretPlace {
        column: "pending_sealed",
        ..HOME
    };
    let moved_table = SecretPlace {
        table: "server_config",
        ..HOME
    };
    for place in [moved_row, moved_column, moved_table] {
        assert_eq!(
            keys.open(&place, &sealed).unwrap_err(),
            SecretError::Rejected
        );
    }
}

#[test]
fn flipped_bit_does_not_open() {
    let keys = keys();
    let sealed = keys.seal(&HOME, &plain()).unwrap();
    for index in 21..sealed.len() {
        let mut altered = sealed.clone();
        if let Some(byte) = altered.get_mut(index) {
            *byte ^= 0x01;
        }
        assert_eq!(
            keys.open(&HOME, &altered).unwrap_err(),
            SecretError::Rejected
        );
    }
}

#[test]
fn another_master_key_does_not_open() {
    let sealed = keys().seal(&HOME, &plain()).unwrap();
    assert_eq!(
        keys().open(&HOME, &sealed).unwrap_err(),
        SecretError::WrongKey
    );
}

#[test]
fn rewritten_key_id_still_does_not_open() {
    let ours = keys();
    let theirs = keys();
    let sealed = theirs.seal(&HOME, &plain()).unwrap();
    let mut forged = vec![1u8];
    forged.extend(ours.key_id());
    forged.extend(sealed.iter().skip(9));
    assert_eq!(
        ours.open(&HOME, &forged).unwrap_err(),
        SecretError::Rejected
    );
}

#[test]
fn same_plaintext_seals_differently_each_time() {
    let keys = keys();
    let first = keys.seal(&HOME, &plain()).unwrap();
    let second = keys.seal(&HOME, &plain()).unwrap();
    assert_ne!(first, second);
}

#[test]
fn sealed_bytes_record_the_version_and_key_id() {
    let keys = keys();
    let sealed = keys.seal(&HOME, &plain()).unwrap();
    assert_eq!(sealed.first(), Some(&1));
    assert_eq!(sealed.get(1..9), Some(keys.key_id().as_slice()));
    assert!(
        !sealed
            .windows(plain().expose().len())
            .any(|w| w == plain().expose())
    );
}

#[test]
fn truncated_or_empty_input_is_an_error_not_empty() {
    let keys = keys();
    let sealed = keys.seal(&HOME, &plain()).unwrap();
    assert!(keys.open(&HOME, &[]).is_err());
    assert!(
        keys.open(&HOME, &sealed.iter().take(20).copied().collect::<Vec<u8>>())
            .is_err()
    );
    let mut wrong_version = sealed.clone();
    *wrong_version.first_mut().unwrap() = 9;
    assert!(matches!(
        keys.open(&HOME, &wrong_version),
        Err(SecretError::Unreadable(_))
    ));
}

#[test]
fn text_round_trips_through_secret() {
    let keys = keys();
    let sealed = keys
        .seal_text(&HOME, &Secret::from_untrusted("proxy-word".to_owned()))
        .unwrap();
    assert_eq!(
        keys.open_text(&HOME, &sealed).unwrap().expose(),
        "proxy-word"
    );
}
