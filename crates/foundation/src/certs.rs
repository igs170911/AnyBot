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
    Other,
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
}

impl TrustStore {
    pub fn new(extra: &TrustedCerts) -> Result<Arc<Self>, String> {
        let mut roots = RootCertStore::empty();
        let loaded = rustls_native_certs::load_native_certs();
        for cert in loaded.certs {
            match roots.add(cert) {
                Ok(()) | Err(Error::InvalidCertificate(_)) => {}
                Err(err) => return Err(format!("os trust store: {err}")),
            }
        }
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
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
        }))
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
    fault_from_text(&err.to_string())
}

fn fault_from_text(text: &str) -> CertFault {
    if text.contains("NotValidForName") {
        return CertFault::NameMismatch;
    }
    if text.contains("NotValidYet") {
        return CertFault::NotValidYet;
    }
    if text.contains("Expired") {
        return CertFault::Expired;
    }
    if text.contains("InvalidCertificate") || text.contains("certificate") {
        return CertFault::Untrusted;
    }
    CertFault::Other
}
