use axum::extract::Request;

use anybot_domain::version_release::{VersionVerdict, judge_client_version};
use anybot_messages::{MessageCode, Params};

use crate::problems::{auth_problem_params, problem_response};
use axum::http::Method;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

const PRE_SESSION_PATHS: [&str; 3] = [
    "/v1/account/login",
    "/v1/account/register",
    "/v1/computers/pair",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreSessionMarker;

#[must_use]
pub fn pre_session_gate(
    state: &crate::app::AppState,
    parts: &mut axum::http::request::Parts,
) -> Option<Response> {
    let raw = crate::auth_version::header_of(&parts.headers);
    let now_ms = i64::try_from(state.clock.now_ms().millis()).unwrap_or(i64::MAX);
    let verdict = judge_client_version(raw.as_deref(), &state.releases, now_ms);
    if verdict == VersionVerdict::Accept {
        return None;
    }
    Some(denied(raw.as_deref()))
}

fn denied(header: Option<&str>) -> Response {
    let shown = header.unwrap_or("unknown");
    let params = Params::new().with_text("version", shown);
    problem_response(auth_problem_params(
        StatusCode::FORBIDDEN.as_u16(),
        MessageCode::InstallVersionOld,
        &params,
    ))
}

pub async fn pre_session_layer(
    axum::extract::State(state): axum::extract::State<crate::app::AppState>,
    request: Request,
    next: Next,
) -> Response {
    if is_pre_session(&request) {
        let (mut parts, body) = request.into_parts();
        if let Some(blocked) = pre_session_gate(&state, &mut parts) {
            crate::auth_csrf::drain_body(body).await;
            return blocked;
        }
        parts.extensions.insert(PreSessionMarker);
        return next.run(Request::from_parts(parts, body)).await;
    }
    next.run(request).await
}

fn is_pre_session(request: &Request) -> bool {
    request.method() == Method::POST && PRE_SESSION_PATHS.contains(&request.uri().path())
}
