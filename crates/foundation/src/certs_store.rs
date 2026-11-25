use super::{TrustStore, TrustedCerts};

#[test]
fn os_store_loads_without_bundled_roots() {
    let built = TrustStore::new(&TrustedCerts::empty()).expect("os trust store");
    assert!(built.os_notes().iter().all(|note| !note.is_empty()));
}

#[test]
fn os_notes_are_readable_when_some_certs_fail() {
    let built = TrustStore::new(&TrustedCerts::empty()).expect("os trust store");
    let _notes = built.os_notes();
}
