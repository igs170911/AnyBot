use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, RootCertStore, SignatureScheme};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertFault {
    Untrusted,
    NameMismatch,
    Expired,
    NotValidYet,
    Other(String),
}

#[derive(Clone, Debug, Default)]
pub struct TrustedCerts {
    ders: Vec<CertificateDer<'static>>,
}

impl TrustedCerts {
    #[must_use]
    pub fn empty() -> Self {
        Self { ders: Vec::new() }
    }

    pub fn parse_pem(pem: &str) -> Result<Self, String> {
        reject_preamble(pem.as_bytes())?;
        let mut reader = std::io::Cursor::new(pem.as_bytes());
        let mut ders = Vec::new();
        let mut saw = false;
        loop {
            match rustls_pki_types::pem::from_buf(&mut reader) {
                Ok(None) => break,
                Ok(Some((kind, der))) => {
                    saw = true;
                    if kind != rustls_pki_types::pem::SectionKind::Certificate {
                        return Err(format!("pem section {kind:?} is not a certificate"));
                    }
                    ders.push(CertificateDer::from(der));
                }
                Err(err) => return Err(format!("pem parse failed: {err}")),
            }
        }
        if !saw {
            return Err("pem contains no certificate".to_owned());
        }
        reject_trailing(pem.as_bytes(), reader.position())?;
        Ok(Self { ders })
    }
}

fn reject_preamble(pem: &[u8]) -> Result<(), String> {
    let Some(start) = find_begin(pem) else {
        return Ok(());
    };
    let head = pem.get(..start).ok_or("pem preamble offset")?;
    if head.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err("pem contains non-certificate content".to_owned());
    }
    Ok(())
}

fn find_begin(pem: &[u8]) -> Option<usize> {
    pem.windows(11).position(|window| window == b"-----BEGIN ")
}

fn reject_trailing(pem: &[u8], position: u64) -> Result<(), String> {
    let offset = usize::try_from(position).map_err(|err| err.to_string())?;
    let rest = pem.get(offset..).ok_or("pem trailing offset")?;
    if rest.iter().any(|byte| !byte.is_ascii_whitespace()) {
        return Err("pem contains non-certificate content".to_owned());
    }
    Ok(())
}

impl TrustedCerts {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ders.is_empty()
    }

    #[must_use]
    pub fn ders(&self) -> &[CertificateDer<'static>] {
        &self.ders
    }
}

#[derive(Debug)]
pub struct TrustStore {
    roots: Arc<RootCertStore>,
    extra: Vec<CertificateDer<'static>>,
    supported: WebPkiSupportedAlgorithms,
    os_notes: Vec<String>,
}

impl TrustStore {
    pub fn new(extra: &TrustedCerts) -> Result<Arc<Self>, String> {
        let loaded = crate::certs_os::load_os_roots();
        if loaded.certs.is_empty() {
            return Err(os_store_error_text(&loaded.notes));
        }
        let mut roots = RootCertStore::empty();
        let mut skipped = loaded.notes;
        for cert in loaded.certs {
            if let Err(err) = roots.add(cert) {
                skipped.push(format!("os trust store: {err}"));
            }
        }
        if roots.is_empty() {
            return Err(os_store_error_text(&skipped));
        }
        for cert in &extra.ders {
            roots
                .add(cert.clone())
                .map_err(|err| format!("trusted certificate rejected: {err}"))?;
        }
        let provider = ring_provider();
        Ok(Arc::new(Self {
            roots: Arc::new(roots),
            extra: extra.ders.clone(),
            supported: provider.signature_verification_algorithms,
            os_notes: skipped,
        }))
    }

    #[must_use]
    pub fn os_notes(&self) -> &[String] {
        &self.os_notes
    }

    #[must_use]
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    pub fn client_config(self: &Arc<Self>) -> Result<rustls::ClientConfig, String> {
        let versions = rustls::ClientConfig::builder_with_provider(ring_provider())
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
            .map_err(|err| format!("tls versions: {err}"))?;
        let built = versions
            .dangerous()
            .with_custom_certificate_verifier(Arc::clone(self) as Arc<dyn ServerCertVerifier>)
            .with_no_client_auth();
        Ok(built)
    }
}

fn os_store_error_text(notes: &[String]) -> String {
    if notes.is_empty() {
        return "os trust store returned no certificates".to_owned();
    }
    format!("os trust store unreadable: {}", notes.join("; "))
}

fn ring_provider() -> Arc<rustls::crypto::CryptoProvider> {
    let provider = rustls::crypto::ring::default_provider();
    let installed = rustls::crypto::CryptoProvider::install_default(provider.clone());
    if installed.is_err() {
        return Arc::new(provider);
    }
    Arc::new(provider)
}

impl ServerCertVerifier for TrustStore {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if self
            .extra
            .iter()
            .any(|trusted| trusted.as_ref() == end_entity.as_ref())
        {
            return crate::certs_check::verify_exact(end_entity, server_name, now, &self.supported);
        }
        let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::clone(&self.roots))
            .build()
            .map_err(|err| Error::General(err.to_string()))?;
        verifier.verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.supported)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.supported)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported.supported_schemes()
    }
}

#[must_use]
pub fn classify_tls(err: &Error) -> CertFault {
    let Error::InvalidCertificate(kind) = err else {
        return CertFault::Other(err.to_string());
    };
    classify_certificate(kind)
}

#[must_use]
pub fn classify_tls_error(err: &(dyn std::error::Error + 'static)) -> Option<CertFault> {
    let mut current = Some(err);
    while let Some(item) = current {
        if let Some(tls) = item.downcast_ref::<Error>() {
            return Some(classify_tls(tls));
        }
        if let Some(io) = item.downcast_ref::<std::io::Error>()
            && let Some(inner) = std::io::Error::get_ref(io)
            && let Some(fault) = classify_tls_error(inner)
        {
            return Some(fault);
        }
        current = std::error::Error::source(item);
    }
    None
}

fn classify_certificate(kind: &rustls::CertificateError) -> CertFault {
    if cert_is_name(kind) {
        return CertFault::NameMismatch;
    }
    if cert_is_not_yet(kind) {
        return CertFault::NotValidYet;
    }
    if cert_is_expired(kind) {
        return CertFault::Expired;
    }
    CertFault::Other(kind.to_string())
}

fn cert_is_name(kind: &rustls::CertificateError) -> bool {
    matches!(
        kind,
        rustls::CertificateError::NotValidForName
            | rustls::CertificateError::NotValidForNameContext { .. }
    )
}

fn cert_is_not_yet(kind: &rustls::CertificateError) -> bool {
    matches!(
        kind,
        rustls::CertificateError::NotValidYet | rustls::CertificateError::NotValidYetContext { .. }
    )
}

fn cert_is_expired(kind: &rustls::CertificateError) -> bool {
    matches!(
        kind,
        rustls::CertificateError::Expired | rustls::CertificateError::ExpiredContext { .. }
    )
}
