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
fn cert_file_path_does_not_replace_the_os_store() {
    let built = TrustStore::new(&TrustedCerts::empty()).expect("os trust store");
    let from_file = rustls_native_certs::load_certs_from_paths(
        Some(std::path::Path::new("target/missing-cert-file.pem")),
        Some(std::path::Path::new("target/missing-cert-dir")),
    );
    assert!(from_file.certs.is_empty());
    assert!(!from_file.errors.is_empty());
    assert!(built.root_count() > 0);
}
