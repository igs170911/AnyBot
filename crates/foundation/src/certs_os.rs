use rustls_pki_types::CertificateDer;

pub(crate) struct OsRoots {
    pub certs: Vec<CertificateDer<'static>>,
    pub notes: Vec<String>,
}

pub(crate) fn load_os_roots() -> OsRoots {
    #[cfg(target_os = "macos")]
    {
        macos::load()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        unix::load()
    }
    #[cfg(windows)]
    {
        windows::load()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use rustls_pki_types::CertificateDer;
    use security_framework::trust_settings::{Domain, TrustSettings, TrustSettingsForCertificate};

    use super::OsRoots;

    pub(super) fn load() -> OsRoots {
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
        OsRoots { certs, notes }
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
}

#[cfg(all(unix, not(target_os = "macos")))]
mod unix {
    use super::OsRoots;

    const BUNDLE_FILES: &[&str] = &[
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/ca-bundle.pem",
        "/etc/pki/tls/cacert.pem",
        "/etc/ssl/cert.pem",
        "/opt/etc/ssl/certs/ca-certificates.crt",
        "/etc/ssl/certs/cacert.pem",
        "/usr/local/etc/ssl/cert.pem",
        "/usr/local/share/certs/ca-root-nss.crt",
        "/etc/openssl/certs/ca-certificates.crt",
        "/etc/certs/ca-certificates.crt",
        "/etc/ssl/cacert.pem",
        "/data/data/com.termux/files/usr/etc/tls/cert.pem",
        "/boot/system/data/ssl/CARootCertificates.pem",
    ];

    const BUNDLE_DIRS: &[&str] = &[
        "/etc/ssl/certs",
        "/etc/pki/tls/certs",
        "/etc/security/certificates",
        "/usr/local/share/certs",
        "/etc/openssl/certs",
        "/etc/certs/CA",
        "/var/ssl/certs",
    ];

    pub(super) fn load() -> OsRoots {
        super::pem::load_from(BUNDLE_FILES, BUNDLE_DIRS)
    }
}

#[cfg(windows)]
mod windows {
    use rustls_pki_types::CertificateDer;
    use schannel::cert_context::ValidUses;
    use schannel::cert_store::CertStore;

    use super::OsRoots;

    const SERVER_AUTH: &str = "1.3.6.1.5.5.7.3.1";

    pub(super) fn load() -> OsRoots {
        let mut notes = Vec::new();
        let mut certs = Vec::new();
        for name in ["ROOT"] {
            match read_store(name) {
                Ok(found) => certs.extend(found),
                Err(err) => notes.push(format!("failed to open {name} certificate store: {err}")),
            }
        }
        if certs.is_empty() && notes.is_empty() {
            notes.push("windows certificate store returned no certificates".to_owned());
        }
        OsRoots { certs, notes }
    }

    fn read_store(name: &str) -> Result<Vec<CertificateDer<'static>>, String> {
        let store = CertStore::open_current_user(name).map_err(|err| err.to_string())?;
        let mut certs = Vec::new();
        for cert in store.certs() {
            if usable(&cert)? {
                certs.push(CertificateDer::from(cert.to_der().to_vec()));
            }
        }
        Ok(certs)
    }

    fn usable(cert: &schannel::cert_context::CertContext) -> Result<bool, String> {
        if !cert.is_time_valid().map_err(|err| err.to_string())? {
            return Ok(false);
        }
        match cert.valid_uses().map_err(|err| err.to_string())? {
            ValidUses::All => Ok(true),
            ValidUses::Oids(oids) => Ok(oids.iter().any(|oid| oid == SERVER_AUTH)),
        }
    }
}

#[cfg(any(test, all(unix, not(target_os = "macos"))))]
pub(crate) mod pem {
    use std::io::Read;
    use std::path::Path;

    use rustls_pki_types::CertificateDer;

    use super::OsRoots;

    pub(crate) fn load_from(files: &[&str], dirs: &[&str]) -> OsRoots {
        let mut notes = Vec::new();
        let mut certs = Vec::new();
        if let Some(path) = first_file(files) {
            match read_pem_file(path) {
                Ok(found) => certs.extend(found),
                Err(err) => notes.push(format!("{}: {err}", path.display())),
            }
        }
        for dir in existing_dirs(dirs) {
            match read_pem_dir(dir) {
                Ok(found) => certs.extend(found),
                Err(err) => notes.push(format!("{}: {err}", dir.display())),
            }
        }
        if certs.is_empty() && notes.is_empty() {
            notes.push("no system certificate bundle found".to_owned());
        }
        certs.sort_by(|left, right| left.as_ref().cmp(right.as_ref()));
        certs.dedup();
        OsRoots { certs, notes }
    }

    fn first_file<'a>(paths: &[&'a str]) -> Option<&'a Path> {
        paths
            .iter()
            .copied()
            .map(Path::new)
            .find(|path| path.is_file())
    }

    fn existing_dirs<'a>(paths: &[&'a str]) -> Vec<&'a Path> {
        paths
            .iter()
            .copied()
            .map(Path::new)
            .filter(|path| path.is_dir())
            .collect()
    }

    fn read_pem_file(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
        let mut file = std::fs::File::open(path).map_err(|err| err.to_string())?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        Ok(pem_certs(&bytes))
    }

    fn read_pem_dir(dir: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
        let entries = std::fs::read_dir(dir).map_err(|err| err.to_string())?;
        let mut certs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            certs.extend(read_pem_file(&path)?);
        }
        Ok(certs)
    }

    fn pem_certs(bytes: &[u8]) -> Vec<CertificateDer<'static>> {
        let mut reader = std::io::Cursor::new(bytes);
        let mut certs = Vec::new();
        loop {
            match rustls_pki_types::pem::from_buf(&mut reader) {
                Ok(Some((rustls_pki_types::pem::SectionKind::Certificate, der))) => {
                    certs.push(CertificateDer::from(der));
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => return certs,
            }
        }
    }
}
