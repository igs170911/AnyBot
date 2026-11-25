use rustls_pki_types::CertificateDer;
use security_framework::trust_settings::{Domain, TrustSettings, TrustSettingsForCertificate};

pub(crate) fn load_os_roots() -> (Vec<CertificateDer<'static>>, Vec<String>) {
    let mut notes = Vec::new();
    let mut trusted = std::collections::BTreeMap::new();
    for domain in [Domain::User, Domain::Admin, Domain::System] {
        collect_domain(domain, &mut notes, &mut trusted);
    }
    let mut certs = Vec::new();
    for (der, setting) in trusted {
        if keep_root(setting) {
            certs.push(CertificateDer::from(der));
        }
    }
    (certs, notes)
}

fn keep_root(setting: TrustSettingsForCertificate) -> bool {
    matches!(
        setting,
        TrustSettingsForCertificate::TrustRoot | TrustSettingsForCertificate::TrustAsRoot
    )
}

fn collect_domain(
    domain: Domain,
    notes: &mut Vec<String>,
    trusted: &mut std::collections::BTreeMap<Vec<u8>, TrustSettingsForCertificate>,
) {
    let settings = TrustSettings::new(domain);
    let iter = match settings.iter() {
        Ok(iter) => iter,
        Err(err) => {
            notes.push(format!("{}: {err}", domain_note(domain)));
            return;
        }
    };
    for cert in iter {
        remember_cert(&settings, &cert, notes, trusted);
    }
}

fn remember_cert(
    settings: &TrustSettings,
    cert: &security_framework::certificate::SecCertificate,
    notes: &mut Vec<String>,
    trusted: &mut std::collections::BTreeMap<Vec<u8>, TrustSettingsForCertificate>,
) {
    let der = cert.to_der();
    let setting = match settings.tls_trust_settings_for_certificate(cert) {
        Ok(Some(setting)) => setting,
        Ok(None) => TrustSettingsForCertificate::TrustRoot,
        Err(err) => {
            notes.push(format!("certificate not trusted: {err}"));
            return;
        }
    };
    let _kept = trusted.entry(der).or_insert(setting);
}

fn domain_note(domain: Domain) -> &'static str {
    match domain {
        Domain::User => "failed to load user trust settings",
        Domain::Admin => "failed to load admin trust settings",
        Domain::System => "failed to load system trust settings",
    }
}
