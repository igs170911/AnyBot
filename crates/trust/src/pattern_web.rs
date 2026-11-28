use url::Url;

pub(crate) fn host_of_url(text: &str) -> Option<String> {
    let url = Url::parse(text).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    host_text(url.host_str()?)
}

pub(crate) fn host_of_domain(domain: &str) -> Option<String> {
    let url = Url::parse(&format!("http://{domain}/")).ok()?;
    host_text(url.host_str()?)
}

fn host_text(host: &str) -> Option<String> {
    let trimmed = host.trim_end_matches('.').to_ascii_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

pub(crate) fn domain_covers(domain: &str, url: &str) -> bool {
    let (Some(wanted), Some(host)) = (host_of_domain(domain), host_of_url(url)) else {
        return false;
    };
    host == wanted || host.ends_with(&format!(".{wanted}"))
}
