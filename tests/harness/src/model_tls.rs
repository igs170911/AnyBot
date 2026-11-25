use std::sync::Arc;

use std::net::IpAddr;

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

pub struct MintedCert {
    pub pem: String,
    pub der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub dns: String,
}

pub struct IssuedLeaf {
    pub cert: MintedCert,
    pub issuer_pem: String,
}

pub fn mint_self_signed(dns: &str, not_after_days: i64) -> Result<MintedCert, String> {
    let key = KeyPair::generate().map_err(|err| err.to_string())?;
    let params = self_signed_params(dns, not_after_days)?;
    let cert = params.self_signed(&key).map_err(|err| err.to_string())?;
    Ok(owned_cert(dns, &cert.pem(), cert.der(), &key))
}

pub fn mint_issued(dns: &str, issuer_dns: &str) -> Result<IssuedLeaf, String> {
    let issuer_key = KeyPair::generate().map_err(|err| err.to_string())?;
    let issuer_params = ca_params(issuer_dns)?;
    let issuer = issuer_params
        .self_signed(&issuer_key)
        .map_err(|err| err.to_string())?;
    let leaf_key = KeyPair::generate().map_err(|err| err.to_string())?;
    let leaf_params = leaf_params(dns)?;
    let signer = Issuer::from_params(&issuer_params, &issuer_key);
    let leaf = leaf_params
        .signed_by(&leaf_key, &signer)
        .map_err(|err| err.to_string())?;
    let cert = owned_cert(dns, &leaf.pem(), leaf.der(), &leaf_key);
    Ok(IssuedLeaf {
        cert,
        issuer_pem: issuer.pem(),
    })
}

pub fn server_config(cert: &MintedCert) -> Result<Arc<ServerConfig>, String> {
    let provider = rustls::crypto::ring::default_provider();
    let _installed = rustls::crypto::CryptoProvider::install_default(provider.clone());
    let provider = Arc::new(provider);
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| err.to_string())?
        .with_no_client_auth()
        .with_single_cert(vec![cert.der.clone()], cert.key_der.clone_key())
        .map_err(|err| err.to_string())?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

fn owned_cert(dns: &str, pem: &str, der: &CertificateDer<'_>, key: &KeyPair) -> MintedCert {
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    MintedCert {
        pem: pem.to_owned(),
        der: der.clone().into_owned(),
        key_der,
        dns: dns.to_owned(),
    }
}

fn self_signed_params(dns: &str, not_after_days: i64) -> Result<CertificateParams, String> {
    let mut params = CertificateParams::new(names_of(dns)).map_err(|err| err.to_string())?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
    ];
    params.distinguished_name = name_of(dns);
    apply_window(&mut params, not_after_days);
    Ok(params)
}

fn ca_params(dns: &str) -> Result<CertificateParams, String> {
    let mut params = CertificateParams::new(vec![dns.to_owned()]).map_err(|err| err.to_string())?;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    params.distinguished_name = name_of(dns);
    apply_window(&mut params, 2);
    Ok(params)
}

fn leaf_params(dns: &str) -> Result<CertificateParams, String> {
    let mut params = CertificateParams::new(names_of(dns)).map_err(|err| err.to_string())?;
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.distinguished_name = name_of(dns);
    apply_window(&mut params, 2);
    Ok(params)
}

fn names_of(dns: &str) -> Vec<String> {
    let mut names = vec![dns.to_owned()];
    if dns.parse::<IpAddr>().is_err() {
        names.push("127.0.0.1".to_owned());
    }
    names
}

fn name_of(dns: &str) -> DistinguishedName {
    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, dns);
    name
}

fn apply_window(params: &mut CertificateParams, not_after_days: i64) {
    let now = time::OffsetDateTime::now_utc();
    if not_after_days < 0 {
        params.not_before = shift(now, time::Duration::days(-3));
        params.not_after = shift(now, time::Duration::days(-1));
        return;
    }
    params.not_before = shift(now, time::Duration::hours(-1));
    params.not_after = shift(now, time::Duration::days(not_after_days));
}

fn shift(now: time::OffsetDateTime, delta: time::Duration) -> time::OffsetDateTime {
    match now.checked_add(delta) {
        Some(next) => next,
        None => now,
    }
}
