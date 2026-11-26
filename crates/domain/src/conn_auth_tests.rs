use crate::conn_auth::{CsrfDecision, CsrfFlags, OriginKey, csrf_check, origin_keys, parse_origin};

fn keys() -> Vec<OriginKey> {
    origin_keys(
        Some("https://s.test"),
        &["127.0.0.1".to_owned(), "::1".to_owned()],
        8731,
    )
}

use crate::conn_auth::CsrfCreds;

fn flags(write: bool, cookie: bool, bearer: bool) -> CsrfFlags {
    CsrfFlags {
        write_method: write,
        csrf_header_ok: true,
        pre_bearer_endpoint: false,
        creds: CsrfCreds {
            has_cookie: cookie,
            has_bearer: bearer,
            wants_cookie: false,
        },
    }
}

#[test]
fn t_conn_auth_1() {
    let keys = keys();
    let https = parse_origin("https://s.test").unwrap();
    let local = parse_origin("http://127.0.0.1:8731").unwrap();
    assert_eq!(https.port, 443);
    assert_eq!(local.port, 8731);
    assert!(keys.contains(&https));
    assert!(keys.contains(&local));
    assert!(keys.contains(&OriginKey {
        scheme: "http".to_owned(),
        host: "localhost".to_owned(),
        port: 8731,
    }));
}

#[test]
fn conn_auth_wildcard_listeners_do_not_expand() {
    let keys = origin_keys(None, &["0.0.0.0".to_owned(), "::".to_owned()], 8731);
    assert!(keys.is_empty());
}

#[test]
fn t_conn_auth_5() {
    let keys = keys();
    let foreign = csrf_check(
        &flags(true, false, false),
        &keys,
        Some("https://other.test"),
        Some("cross-site"),
    );
    assert_eq!(foreign, CsrfDecision::Deny);
    let preflight = csrf_check(
        &flags(true, false, false),
        &keys,
        Some("https://other.test"),
        None,
    );
    assert_eq!(preflight, CsrfDecision::Deny);
    let same = csrf_check(
        &flags(true, false, false),
        &keys,
        Some("https://s.test"),
        None,
    );
    assert_eq!(same, CsrfDecision::Allow);
}

#[test]
fn conn_auth_cross_site_without_origin_denies() {
    let keys = keys();
    let got = csrf_check(&flags(true, false, false), &keys, None, Some("cross-site"));
    assert_eq!(got, CsrfDecision::Deny);
    let got = csrf_check(&flags(true, false, false), &keys, None, Some("same-origin"));
    assert_eq!(got, CsrfDecision::Allow);
}

#[test]
fn conn_auth_native_cookie_writes_denied() {
    let keys = keys();
    let got = csrf_check(&flags(true, true, false), &keys, None, None);
    assert_eq!(got, CsrfDecision::Deny);
    let got = csrf_check(&flags(true, false, true), &keys, None, None);
    assert_eq!(got, CsrfDecision::Allow);
    let wants = CsrfFlags {
        creds: CsrfCreds {
            has_cookie: false,
            has_bearer: false,
            wants_cookie: true,
        },
        ..flags(true, false, false)
    };
    let got = csrf_check(&wants, &keys, None, None);
    assert_eq!(got, CsrfDecision::Deny);
}

#[test]
fn conn_auth_writes_need_the_csrf_header() {
    let keys = keys();
    let weak = CsrfFlags {
        csrf_header_ok: false,
        ..flags(true, true, false)
    };
    let got = csrf_check(
        &weak,
        &keys,
        Some("http://127.0.0.1:8731"),
        Some("same-origin"),
    );
    assert_eq!(got, CsrfDecision::Deny);
}

#[test]
fn conn_auth_cookie_bearer_mix_denied() {
    let keys = keys();
    let got = csrf_check(
        &flags(true, true, true),
        &keys,
        Some("https://s.test"),
        None,
    );
    assert_eq!(got, CsrfDecision::Deny);
}

#[test]
fn conn_auth_get_sse_cross_site_cookie_denied() {
    let keys = keys();
    let got = csrf_check(&flags(false, true, false), &keys, None, Some("cross-site"));
    assert_eq!(got, CsrfDecision::Deny);
    let got = csrf_check(&flags(false, true, false), &keys, None, Some("same-origin"));
    assert_eq!(got, CsrfDecision::Allow);
    let got = csrf_check(&flags(false, true, false), &keys, None, None);
    assert_eq!(got, CsrfDecision::Allow);
}

#[test]
fn conn_auth_unparsable_origin_denied() {
    let keys = keys();
    let got = csrf_check(&flags(true, false, false), &keys, Some("not a url"), None);
    assert_eq!(got, CsrfDecision::Deny);
}
