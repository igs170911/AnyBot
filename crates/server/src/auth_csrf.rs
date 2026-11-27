use axum::extract::Request;
use axum::http::HeaderName;
use axum::http::Method;
use axum::http::header::ORIGIN;
use axum::middleware::Next;
use axum::response::Response;

use anybot_domain::conn_auth::{CsrfFlags, OriginKey, csrf_check};

use crate::app::AppState;

const SEC_FETCH_SITE: HeaderName = HeaderName::from_static("sec-fetch-site");
use crate::auth_credential::{cookie_secret, csrf_header_ok};
use crate::problems::{auth_problem, problem_response};

pub async fn origin_keys_of(state: &AppState) -> Vec<OriginKey> {
    let config = state.config_view().await;
    let mut keys = anybot_domain::conn_auth::origin_keys(
        config.public_url.as_deref(),
        &config.listen_addresses,
        config.listen_port,
    );
    for port in state.listen_ports.iter().copied() {
        let extra = anybot_domain::conn_auth::origin_keys(None, &config.listen_addresses, port);
        keys.extend(extra);
    }
    keys
}

#[must_use]
pub fn origin_view(keys: &[OriginKey], origin: Option<&str>) -> OriginView {
    let Some(text) = origin else {
        return OriginView {
            secure_flag: false,
            present: false,
        };
    };
    OriginView {
        secure_flag: origin_allowed_https(keys, text),
        present: true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginView {
    pub secure_flag: bool,
    pub present: bool,
}

pub async fn credential_view(state: &AppState, origin: Option<&str>) -> OriginView {
    origin_view(&origin_keys_of(state).await, origin)
}

fn origin_allowed_https(keys: &[OriginKey], origin: &str) -> bool {
    anybot_domain::conn_auth::parse_origin(origin)
        .is_some_and(|key| key.scheme == "https" && keys.contains(&key))
}

const PRE_BEARER_PATHS: [&str; 4] = [
    "/v1/account/login",
    "/v1/account/register",
    "/v1/account/login/two-step",
    "/v1/computers/pair",
];
const CREDENTIAL_QUERY_NAMES: [&str; 4] = ["session_key", "token", "bearer", "invite"];

pub async fn csrf_layer(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if credential_in_query(request.uri().query()) {
        drain_body(request.into_body()).await;
        return denied(401, anybot_messages::MessageCode::AccountLoginCredentials);
    }
    let (parts, mut body) = request.into_parts();
    let decision = gate_of(&parts, &mut body, &state).await;
    if decision == anybot_domain::conn_auth::CsrfDecision::Deny {
        drain_body(body).await;
        return denied(403, anybot_messages::MessageCode::AccountLoginCredentials);
    }
    next.run(Request::from_parts(parts, body)).await
}

async fn gate_of(
    parts: &axum::http::request::Parts,
    body: &mut axum::body::Body,
    state: &AppState,
) -> anybot_domain::conn_auth::CsrfDecision {
    let keys = origin_keys_of(state).await;
    let headers = &parts.headers;
    let origin = headers.get(ORIGIN).and_then(|v| v.to_str().ok());
    let sec_fetch = headers.get(&SEC_FETCH_SITE).and_then(|v| v.to_str().ok());
    let wants_cookie = parts.method == Method::POST
        && parts.uri.path() == "/v1/account/login"
        && body_wants_cookie(body).await;
    let flags = CsrfFlags {
        write_method: !is_read_method(&parts.method),
        csrf_header_ok: csrf_header_ok(headers),
        pre_bearer_endpoint: PRE_BEARER_PATHS.contains(&parts.uri.path()),
        creds: anybot_domain::conn_auth::CsrfCreds {
            has_cookie: cookie_secret(headers).is_some(),
            has_bearer: headers.get(axum::http::header::AUTHORIZATION).is_some(),
            wants_cookie,
        },
    };
    csrf_check(&flags, &keys, origin, sec_fetch)
}

fn is_read_method(method: &Method) -> bool {
    method == Method::GET || method == Method::HEAD
}

async fn body_wants_cookie(body: &mut axum::body::Body) -> bool {
    let Some(bytes) = buffered_bytes(body).await else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    value
        .get("credential")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| text == "cookie")
}

async fn buffered_bytes(body: &mut axum::body::Body) -> Option<Vec<u8>> {
    let collected = http_body_util::BodyExt::collect(&mut *body).await.ok()?;
    let bytes = collected.to_bytes();
    if bytes.len() > 4096 {
        return None;
    }
    let rest = axum::body::Body::from(bytes.clone());
    *body = rest;
    Some(bytes.to_vec())
}

pub(crate) async fn drain_body(body: axum::body::Body) {
    match axum::body::to_bytes(body, crate::req_write::MAX_WRITE_BODY).await {
        Ok(bytes) => drop(bytes),
        Err(error) => drop(error),
    }
}

fn credential_in_query(query: Option<&str>) -> bool {
    let Some(query) = query else {
        return false;
    };
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, value)| CREDENTIAL_QUERY_NAMES.contains(&name) && !value.is_empty())
}

fn denied(status: u16, code: anybot_messages::MessageCode) -> Response {
    problem_response(auth_problem(status, code))
}
