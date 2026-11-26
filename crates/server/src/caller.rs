use axum::extract::{ConnectInfo, FromRequestParts, Request};
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::IntoResponse;

use std::net::{IpAddr, SocketAddr};

use anybot_domain::session_term::{session_expired, touch_due};
use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::ProblemJson;
use anybot_store_server::{accounts, sessions};

use crate::app::AppState;
use crate::pass_hash::key_digest;
use crate::problems::{auth_problem, problem_response};
use crate::req_source::request_source_address;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    Cookie,
    Bearer,
}

pub struct Caller {
    pub account: String,
    pub admin: bool,
    pub session_id: String,
    pub device_id: String,
    pub credential: CredentialKind,
    pub address: String,
}

#[derive(Debug)]
pub struct CallerDenied {
    pub problem: Box<ProblemJson>,
}

impl IntoResponse for CallerDenied {
    fn into_response(self) -> axum::response::Response {
        problem_response(self.problem)
    }
}

impl FromRequestParts<AppState> for Caller {
    type Rejection = CallerDenied;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let address = source_of(parts, state)?;
        let bearer = bearer_secret(parts);
        let cookie = crate::auth_credential::cookie_secret(&parts.headers);
        let credential = match (bearer, cookie) {
            (Some(_), Some(_)) => {
                return Err(denied_code(
                    StatusCode::FORBIDDEN,
                    MessageCode::AccountLoginCredentials,
                ))
            }
            (Some(key), None) => (CredentialKind::Bearer, key),
            (None, Some(key)) => (CredentialKind::Cookie, key),
            (None, None) => {
                return Err(denied_code(
                    StatusCode::UNAUTHORIZED,
                    MessageCode::AccountLoginCredentials,
                ))
            }
        };
        let pool = state
            .pool
            .as_ref()
            .as_ref()
            .ok_or_else(|| internal_static("no database pool"))?;
        let row = sessions::find_session_by_key_hash(pool, &key_digest(&credential.1))
            .await
            .map_err(internal)?;
        let row = row.ok_or(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ))?;
        let now_ms = state.clock.now_ms().millis();
        let last_active =
            u64::try_from(row.last_active_ms).map_err(|e| internal_detail(&e.to_string()))?;
        if session_expired(last_active, now_ms) {
            return Err(denied_code(
                StatusCode::UNAUTHORIZED,
                MessageCode::AccountLoginCredentials,
            ));
        }
        let account = accounts::find_account(pool, &row.account)
            .await
            .map_err(internal)?;
        let account = account.ok_or(denied_code(
            StatusCode::UNAUTHORIZED,
            MessageCode::AccountLoginCredentials,
        ))?;
        if account.disabled {
            return Err(denied_code(
                StatusCode::UNAUTHORIZED,
                MessageCode::AccountLoginDisabled,
            ));
        }
        if row.last_password_at_ms != account.last_password_at_ms {
            return Err(denied_code(
                StatusCode::UNAUTHORIZED,
                MessageCode::AccountLoginCredentials,
            ));
        }
        let Some(device_id) = row.device_id.clone() else {
            return Err(denied_code(
                StatusCode::UNAUTHORIZED,
                MessageCode::AccountLoginCredentials,
            ));
        };
        touch_if_due(pool, &row, now_ms).await?;
        Ok(Caller {
            account: row.account,
            admin: account.admin,
            session_id: row.id,
            device_id,
            credential: credential.0,
            address,
        })
    }
}

fn bearer_secret(parts: &Parts) -> Option<Secret> {
    let raw = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let key = raw.strip_prefix("Bearer ")?;
    if key.is_empty() {
        return None;
    }
    Some(Secret::from_untrusted(key.to_owned()))
}

fn source_of(parts: &Parts, state: &AppState) -> Result<String, CallerDenied> {
    let peer = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or_else(|| internal_detail("missing connect info"))?
        .0
        .ip();
    let request = Request::from_parts(parts.clone(), axum::body::Body::empty());
    Ok(request_source_address(&request, &peer_string(peer), state))
}

fn peer_string(peer: IpAddr) -> String {
    peer.to_string()
}

async fn touch_if_due(
    pool: &anybot_store_server::Pool,
    row: &sessions::SessionRow,
    now_ms: u64,
) -> Result<(), CallerDenied> {
    let last = u64::try_from(row.last_active_ms).map_err(|e| internal_detail(&e.to_string()))?;
    let at = i64::try_from(now_ms).map_err(|e| internal_detail(&e.to_string()))?;
    if touch_due(last, now_ms) {
        sessions::touch_session(pool, &row.id, at)
            .await
            .map_err(internal)?;
    }
    Ok(())
}

#[must_use]
pub fn denied_code(status: StatusCode, code: MessageCode) -> CallerDenied {
    CallerDenied {
        problem: auth_problem(status.as_u16(), code),
    }
}

fn internal_static(detail: &'static str) -> CallerDenied {
    CallerDenied {
        problem: Box::new(ProblemJson::new(
            500,
            detail.to_owned(),
            format!("{}/internal", anybot_protocol::PROBLEM_TYPE_BASE),
            anybot_protocol::PROBLEM_CODE_INTERNAL.to_owned(),
        )),
    }
}

fn internal(err: sqlx::Error) -> CallerDenied {
    let detail = err.to_string();
    drop(err);
    internal_detail(&detail)
}

#[must_use]
pub fn internal_detail(detail: &str) -> CallerDenied {
    CallerDenied {
        problem: Box::new(ProblemJson::new(
            500,
            detail.to_owned(),
            format!("{}/internal", anybot_protocol::PROBLEM_TYPE_BASE),
            anybot_protocol::PROBLEM_CODE_INTERNAL.to_owned(),
        )),
    }
}
