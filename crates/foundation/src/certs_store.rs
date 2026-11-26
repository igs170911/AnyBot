use super::{TrustStore, TrustedCerts};

#[test]
fn os_store_loads_without_bundled_roots() {
    let built = TrustStore::new(&TrustedCerts::empty()).expect("os trust store");
    assert!(built.root_count() > 0);
    assert!(built.os_notes().iter().all(|note| !note.is_empty()));
}

#[test]
fn os_notes_are_readable_when_some_certs_fail() {
    let built = TrustStore::new(&TrustedCerts::empty()).expect("os trust store");
    assert!(built.root_count() > 0);
    for note in built.os_notes() {
        assert!(
            note.starts_with("os trust store:")
                || note.starts_with("failed to load")
                || note.starts_with("certificate not trusted:")
        );
    }
}

#[test]
fn pem_text_outside_blocks_is_rejected() {
    let body = "not a certificate\n-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\ntrailer\n";
    let err = TrustedCerts::parse_pem(body).expect_err("preamble");
    assert!(err.contains("non-certificate"));
}

#[test]
fn unreadable_system_bundle_is_an_error() {
    let loaded = crate::certs_os::pem::load_from(
        &["target/m7a5-missing-bundle.pem"],
        &["target/m7a5-missing-bundle-dir"],
    );
    assert!(loaded.certs.is_empty());
    assert!(
        loaded
            .notes
            .iter()
            .any(|note| note.contains("no system certificate bundle found")),
        "{:?}",
        loaded.notes
    );
}
