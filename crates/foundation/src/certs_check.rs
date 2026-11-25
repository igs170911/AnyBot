use rustls::Error;
use rustls::client::danger::ServerCertVerified;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use webpki::EndEntityCert;

pub(crate) fn verify_exact(
    end_entity: &CertificateDer<'_>,
    server_name: &ServerName<'_>,
    now: UnixTime,
    supported: &rustls::crypto::WebPkiSupportedAlgorithms,
) -> Result<ServerCertVerified, Error> {
    let parsed = EndEntityCert::try_from(end_entity).map_err(map_webpki)?;
    parsed
        .verify_is_valid_for_subject_name(server_name)
        .map_err(map_webpki)?;
    check_time(end_entity, now)?;
    if supported.supported_schemes().is_empty() {
        return Err(Error::General("no signature schemes".to_owned()));
    }
    Ok(ServerCertVerified::assertion())
}

fn check_time(end_entity: &CertificateDer<'_>, now: UnixTime) -> Result<(), Error> {
    let parsed = x509_parser::parse_x509_certificate(end_entity.as_ref())
        .map_err(|err| Error::General(err.to_string()))?
        .1;
    let validity = parsed.validity();
    let seconds = i64::try_from(now.as_secs()).map_err(|err| Error::General(err.to_string()))?;
    let current = x509_parser::time::ASN1Time::from_timestamp(seconds)
        .map_err(|err| Error::General(err.to_string()))?;
    if !validity.is_valid_at(current) {
        if current < validity.not_before {
            return Err(Error::InvalidCertificate(
                rustls::CertificateError::NotValidYet,
            ));
        }
        return Err(Error::InvalidCertificate(rustls::CertificateError::Expired));
    }
    Ok(())
}

fn map_webpki(err: webpki::Error) -> Error {
    if webpki_is_name(&err) {
        return Error::InvalidCertificate(rustls::CertificateError::NotValidForName);
    }
    if webpki_is_not_yet(&err) {
        return Error::InvalidCertificate(rustls::CertificateError::NotValidYet);
    }
    if webpki_is_expired(&err) {
        return Error::InvalidCertificate(rustls::CertificateError::Expired);
    }
    Error::InvalidCertificate(rustls::CertificateError::Other(rustls::OtherError(
        std::sync::Arc::new(err),
    )))
}

fn webpki_is_name(err: &webpki::Error) -> bool {
    matches!(err, webpki::Error::CertNotValidForName(_))
}

fn webpki_is_not_yet(err: &webpki::Error) -> bool {
    matches!(err, webpki::Error::CertNotValidYet { .. })
}

fn webpki_is_expired(err: &webpki::Error) -> bool {
    matches!(err, webpki::Error::CertExpired { .. })
}
