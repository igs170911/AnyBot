use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring::default_provider;
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error, SignatureScheme};
use rustls_pki_types::pem::{Error as PemError, PemObject};

#[derive(Clone, Debug)]
pub struct PinnedVerifier {
    pinned: CertificateDer<'static>,
    supported: WebPkiSupportedAlgorithms,
}

impl PinnedVerifier {
    pub fn from_pem(pem: &str) -> Result<Self, String> {
        let mut certs = CertificateDer::pem_reader_iter(pem.as_bytes());
        let first = certs
            .next()
            .ok_or("ca_file contains no certificate")?
            .map_err(|e: PemError| format!("parse ca_file failed: {e}"))?;
        let trailing = certs.next().is_some();
        if trailing {
            return Err("ca_file must contain exactly one certificate".to_owned());
        }
        Ok(Self {
            pinned: CertificateDer::from(first.to_vec()),
            supported: default_provider().signature_verification_algorithms,
        })
    }
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        if *end_entity == self.pinned {
            return Ok(ServerCertVerified::assertion());
        }
        Err(Error::General(
            "server certificate does not match the pinned ca_file certificate".to_owned(),
        ))
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

pub fn pinned_client_config(pem: &str) -> Result<ClientConfig, String> {
    let verifier = PinnedVerifier::from_pem(pem)?;
    Ok(
        ClientConfig::builder_with_provider(Arc::new(default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
            .map_err(|e| format!("pick TLS protocol versions failed: {e}"))?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth(),
    )
}
