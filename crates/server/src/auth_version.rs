use axum::extract::Request;
use axum::http::HeaderName;
use axum::http::Method;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

use anybot_domain::version_release::{VersionVerdict, judge_client_version};
use anybot_messages::{MessageCode, Params};

use crate::problems::{auth_problem_params, problem_response};

pub const CLIENT_VERSION_HEADER: &str = "anybot-client-version";

pub const VERSION_EXEMPT_PATHS: [&str; 2] = ["/v1/health", "/v1/server/status"];

fn header_name() -> HeaderName {
    HeaderName::from_static(CLIENT_VERSION_HEADER)
}

#[must_use]
pub fn header_of(headers: &axum::http::HeaderMap) -> Option<String> {
    let text = headers.get(header_name())?.to_str().ok()?;
    Some(text.to_owned())
}

#[must_use]
fn exempt(path: &str) -> bool {
    VERSION_EXEMPT_PATHS.contains(&path)
}

#[must_use]
fn pre_session(method: &Method, path: &str) -> bool {
    method == Method::POST
        && ["v1/account/login", "v1/account/register"].contains(&path.trim_start_matches('/'))
}

#[must_use]
fn passes(verdict: VersionVerdict, method: &Method, path: &str) -> bool {
    if pre_session(method, path) {
        return true;
    }
    verdict == VersionVerdict::Accept
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

pub async fn version_layer(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    request: Request,
    next: Next,
) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path().to_owned();
    let method = parts.method.clone();
    let raw = header_of(&parts.headers);
    let now_ms = i64::try_from(state.clock.now_ms().millis()).unwrap_or(i64::MAX);
    let verdict = judge_client_version(raw.as_deref(), &state.releases, now_ms);
    if exempt(&path) || passes(verdict, &method, &path) {
        let request = Request::from_parts(parts, body);
        return next.run(request).await;
    }
    crate::auth_csrf::drain_body(body).await;
    denied(raw.as_deref())
}
