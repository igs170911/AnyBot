use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::HeaderName;
use axum::response::{IntoResponse, Response};

use std::net::SocketAddr;

use anybot_messages::MessageCode;
use anybot_protocol::{PROBLEM_JSON_CONTENT_TYPE, PROBLEM_TYPE_BASE, ProblemJson};

use crate::app::AppState;
pub use crate::caller::CallerDenied;
use crate::idem::IDEMPOTENCY_HEADER;
use crate::problems::{auth_problem, status_of};
use crate::req_source::{FORWARDED_HEADER, request_source_address};
use anybot_store_server::Pool;

#[must_use]
pub fn denied(status: StatusCode, code: MessageCode) -> CallerDenied {
    CallerDenied {
        problem: auth_problem(status.as_u16(), code),
    }
}

#[must_use]
pub fn internal_sqlx(err: sqlx::Error) -> CallerDenied {
    let detail = err.to_string();
    drop(err);
    internal_detail(&detail)
}

pub fn internal_text(err: impl std::fmt::Display) -> CallerDenied {
    internal_detail(&err.to_string())
}

#[must_use]
pub fn internal_detail(detail: &str) -> CallerDenied {
    crate::caller::internal_detail(detail)
}

pub fn pool_of(state: &AppState) -> Result<&Pool, CallerDenied> {
    state
        .pool
        .as_ref()
        .as_ref()
        .ok_or_else(|| internal_detail("no database pool"))
}

pub fn idempotency_key(request: &Request) -> Result<String, CallerDenied> {
    let raw = request.headers().get(IDEMPOTENCY_HEADER).ok_or(denied(
        StatusCode::BAD_REQUEST,
        MessageCode::AccountIdempotencyMissing,
    ))?;
    let text = raw.to_str().map_err(|e| internal_detail(&e.to_string()))?;
    match anybot_protocol::IdempotencyKey::parse(text) {
        Ok(key) => Ok(key.as_str().to_owned()),
        Err(reason) => Err(denied_detail(StatusCode::BAD_REQUEST, &reason)),
    }
}

#[must_use]
pub fn denied_detail(status: StatusCode, detail: &str) -> CallerDenied {
    CallerDenied {
        problem: Box::new(ProblemJson::new(
            status.as_u16(),
            detail.to_owned(),
            format!("{PROBLEM_TYPE_BASE}/bad-request"),
            anybot_protocol::PROBLEM_CODE_BAD_REQUEST.to_owned(),
        )),
    }
}

pub async fn read_json<T: serde::de::DeserializeOwned>(
    request: Request,
) -> Result<(T, Vec<u8>), CallerDenied> {
    let raw = axum::body::to_bytes(request.into_body(), MAX_WRITE_BODY).await;
    let raw = raw.map_err(|e| internal_detail(&e.to_string()))?;
    let parsed = serde_json::from_slice::<T>(&raw)
        .map_err(|e| denied_detail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    Ok((parsed, raw.to_vec()))
}

const MAX_WRITE_BODY: usize = 256 * 1024;

pub fn peer_address(request: &Request) -> Result<String, CallerDenied> {
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .ok_or_else(|| internal_detail("missing connect info"))
}

pub fn path_account(request: &Request) -> String {
    request
        .uri()
        .path()
        .rsplit('/')
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

pub fn path_tail(request: &Request) -> String {
    request
        .uri()
        .path()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned()
}

pub fn request_address(request: &Request, state: &AppState) -> Result<String, CallerDenied> {
    let peer = peer_address(request)?;
    Ok(request_source_address(request, &peer, state))
}

pub fn has_forwarded_header(request: &Request) -> bool {
    request.headers().contains_key(FORWARDED_HEADER)
}

#[must_use]
pub fn problem(status: StatusCode, code: MessageCode) -> Response {
    problem_box(auth_problem(status.as_u16(), code))
}

#[must_use]
pub fn problem_box(problem: Box<ProblemJson>) -> Response {
    (
        status_of(&problem),
        [(
            HeaderName::from_static("content-type"),
            PROBLEM_JSON_CONTENT_TYPE.to_owned(),
        )],
        axum::Json(*problem),
    )
        .into_response()
}

#[must_use]
pub fn json_response(status: StatusCode, value: serde_json::Value) -> Response {
    (status, axum::Json(value)).into_response()
}

pub fn value_of<T: serde::Serialize>(item: &T) -> Result<serde_json::Value, CallerDenied> {
    serde_json::to_value(item).map_err(|e| internal_detail(&e.to_string()))
}

pub struct Outcome<'a> {
    pub key: &'a str,
    pub raw: &'a [u8],
    pub scope: &'a str,
    pub status: StatusCode,
    pub value: &'a serde_json::Value,
    pub marker: bool,
}

pub async fn record_outcome(
    pool: &Pool,
    state: &AppState,
    rec: &Outcome<'_>,
) -> Result<Response, CallerDenied> {
    let body_text = outcome_body_text(rec)?;
    let at_ms = i64::try_from(state.clock.now_ms().millis()).map_err(internal_text)?;
    crate::idem::record_result(
        pool,
        rec.key,
        rec.scope,
        &crate::idem::safe_digest(rec.raw),
        rec.status.as_u16(),
        &body_text,
        at_ms,
    )
    .await
    .map_err(internal_sqlx)?;
    Ok(json_response(rec.status, rec.value.clone()))
}

fn outcome_body_text(rec: &Outcome<'_>) -> Result<String, CallerDenied> {
    if rec.marker {
        return Ok(crate::idem::DID_IT_BODY.to_owned());
    }
    serde_json::to_string(rec.value).map_err(internal_text)
}

pub fn caller_scope(request: &Request, identity: &str) -> String {
    crate::idem::request_scope(request.method(), &path_of(request), identity)
}

pub fn path_of(request: &Request) -> String {
    request.uri().path().to_owned()
}
