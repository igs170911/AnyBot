use rustls::CertificateError;
use rustls::Error;

use super::{CertFault, classify_tls};

#[test]
fn untrusted_certificate_keeps_the_issuer_kind() {
    let kind = CertificateError::UnknownIssuer;
    let err = Error::InvalidCertificate(kind.clone());
    assert_eq!(classify_tls(&err), CertFault::Other(kind.to_string()));
}

#[test]
fn name_mismatch_reads_the_certificate_kind() {
    let err = Error::InvalidCertificate(CertificateError::NotValidForName);
    assert_eq!(classify_tls(&err), CertFault::NameMismatch);
}

#[test]
fn not_yet_valid_reads_the_certificate_kind() {
    let err = Error::InvalidCertificate(CertificateError::NotValidYet);
    assert_eq!(classify_tls(&err), CertFault::NotValidYet);
}

#[test]
fn expired_reads_the_certificate_kind() {
    let err = Error::InvalidCertificate(CertificateError::Expired);
    assert_eq!(classify_tls(&err), CertFault::Expired);
}

#[test]
fn unknown_certificate_kind_keeps_the_original() {
    let kind = CertificateError::ApplicationVerificationFailure;
    let err = Error::InvalidCertificate(kind.clone());
    assert_eq!(classify_tls(&err), CertFault::Other(kind.to_string()));
    let handshake = Error::DecryptError;
    assert_eq!(
        classify_tls(&handshake),
        CertFault::Other(handshake.to_string())
    );
}
