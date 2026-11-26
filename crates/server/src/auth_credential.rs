use axum::http::HeaderMap;
use axum::http::header::COOKIE;

use anybot_foundation::Secret;

use crate::auth_csrf::OriginView;

pub const SESSION_COOKIE_NAME: &str = "anybot_session";
pub const CSRF_HEADER_NAME: &str = "anybot-csrf";
pub const CSRF_HEADER_VALUE: &str = "1";

pub fn cookie_secret(headers: &HeaderMap) -> Option<Secret> {
    let raw = headers.get(COOKIE)?.to_str().ok()?;
    for item in raw.split(';') {
        let item = item.trim();
        let Some((name, value)) = item.split_once('=') else {
            continue;
        };
        if name.trim() == SESSION_COOKIE_NAME && !value.trim().is_empty() {
            return Some(Secret::from_untrusted(value.trim().to_owned()));
        }
    }
    None
}

pub fn cookie_issue(origin: &OriginView, key: &Secret, max_age: u64) -> String {
    let mut text = format!(
        "{SESSION_COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age}",
        key.expose(),
    );
    if origin.secure_flag {
        text.push_str("; Secure");
    }
    text
}

pub fn cookie_clear(origin: &OriginView) -> String {
    let mut text = format!(
        "{SESSION_COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
    );
    if origin.secure_flag {
        text.push_str("; Secure");
    }
    text
}

pub fn csrf_header_ok(headers: &HeaderMap) -> bool {
    headers
        .get(CSRF_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == CSRF_HEADER_VALUE)
}
