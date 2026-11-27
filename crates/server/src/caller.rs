use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::StatusCode;
use std::net::{IpAddr, SocketAddr};

use anybot_domain::session_term::touch_due;

use anybot_store_server::sessions;

use crate::caller_session::resolve_session;
use crate::req_source::request_source_address;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::response::IntoResponse;

use anybot_foundation::Secret;
use anybot_messages::MessageCode;
use anybot_protocol::ProblemJson;

use crate::app::AppState;
use crate::problems::{auth_problem, problem_response};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    Cookie,
    Bearer,
    Computer,
    Runner,
}

impl Caller {
    pub(crate) fn guest(address: String) -> Caller {
        Caller {
            account: String::new(),
            admin: false,
            session_id: String::new(),
            device_id: String::new(),
            credential: CredentialKind::Bearer,
            address,
            paired: false,
        }
    }
}

pub struct Caller {
    pub account: String,
    pub admin: bool,
    pub session_id: String,
    pub device_id: String,
    pub credential: CredentialKind,
    pub address: String,
    pub paired: bool,
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
        let address = source_of(parts, state).await?;
        if parts
            .extensions
            .get::<crate::auth_pre::PreSessionMarker>()
            .is_some()
        {
            return Ok(Caller::guest(address));
        }
        let bearer = bearer_secret(parts);
        let cookie = crate::auth_credential::cookie_secret(&parts.headers);
        let credential = match (bearer, cookie) {
            (Some(_), Some(_)) => {
                return Err(denied_code(
                    StatusCode::FORBIDDEN,
                    MessageCode::AccountLoginCredentials,
                ));
            }
            (Some(key), None) => (CredentialKind::Bearer, key),
            (None, Some(key)) => (CredentialKind::Cookie, key),
            (None, None) => {
                return Err(denied_code(
                    StatusCode::UNAUTHORIZED,
                    MessageCode::AccountLoginCredentials,
                ));
            }
        };
        let caller = resolve_session(credential, &address, state).await?;
        if caller.credential == CredentialKind::Runner
            && !anybot_domain::runner_rules::runner_may_call(parts.uri.path())
        {
            return Err(denied_code(
                StatusCode::FORBIDDEN,
                MessageCode::DeviceNoPermission,
            ));
        }
        Ok(caller)
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

async fn source_of(parts: &Parts, state: &AppState) -> Result<String, CallerDenied> {
    let peer = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .ok_or_else(|| internal_detail("missing connect info"))?
        .0
        .ip();
    Ok(request_source_address(&parts.headers, &peer_string(peer), state).await)
}

fn peer_string(peer: IpAddr) -> String {
    peer.to_string()
}

pub(crate) async fn touch_if_due(
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
pub(crate) fn denied_code(status: StatusCode, code: MessageCode) -> CallerDenied {
    CallerDenied {
        problem: auth_problem(status.as_u16(), code),
    }
}

pub(crate) fn internal_static(detail: &'static str) -> CallerDenied {
    CallerDenied {
        problem: Box::new(ProblemJson::new(
            500,
            detail.to_owned(),
            format!("{}/internal", anybot_protocol::PROBLEM_TYPE_BASE),
            anybot_protocol::PROBLEM_CODE_INTERNAL.to_owned(),
        )),
    }
}

pub(crate) fn internal(err: sqlx::Error) -> CallerDenied {
    let detail = err.to_string();
    drop(err);
    internal_detail(&detail)
}

#[must_use]
pub(crate) fn internal_detail(detail: &str) -> CallerDenied {
    CallerDenied {
        problem: Box::new(ProblemJson::new(
            500,
            detail.to_owned(),
            format!("{}/internal", anybot_protocol::PROBLEM_TYPE_BASE),
            anybot_protocol::PROBLEM_CODE_INTERNAL.to_owned(),
        )),
    }
}
