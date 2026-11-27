use axum::http::header::SET_COOKIE;
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;

use anybot_foundation::Secret;
use anybot_protocol::LoginSession;
use anybot_store_server::Pool;

use crate::api_login_flow::open_session;
use crate::app::AppState;
use crate::caller::CallerDenied;
use crate::req_write::{Outcome, internal_detail, record_outcome, value_of};

const SESSION_COOKIE_MAX_AGE: u64 = anybot_domain::session_term::SESSION_IDLE_MS / 1000;

pub(crate) struct SessionIssue<'a> {
    pub state: &'a AppState,
    pub pool: &'a Pool,
    pub account: &'a str,
    pub label: &'a str,
    pub cookie_mode: bool,
    pub computer: bool,
    pub used_recovery: bool,
    pub origin: Option<&'a str>,
    pub key: &'a str,
    pub raw: &'a [u8],
    pub scope: &'a str,
}

pub(crate) async fn issue_session(issue: &SessionIssue<'_>) -> Result<Response, CallerDenied> {
    let session = if issue.computer {
        crate::api_login_computer::open_computer(issue).await?
    } else {
        open_session(
            issue.state,
            issue.pool,
            issue.label,
            issue.account,
            issue.cookie_mode,
            issue.used_recovery,
        )
        .await?
    };
    let value = value_of(&session)?;
    let answer = record_outcome(
        issue.pool,
        issue.state,
        &Outcome {
            key: issue.key,
            raw: issue.raw,
            scope: issue.scope,
            status: StatusCode::OK,
            value: &value,
            marker: true,
        },
    )
    .await?;
    issue_cookie(answer, issue, &session).await
}

async fn issue_cookie(
    answer: Response,
    issue: &SessionIssue<'_>,
    session: &LoginSession,
) -> Result<Response, CallerDenied> {
    if !issue.cookie_mode {
        return Ok(answer);
    }
    let raw = session
        .session_key
        .clone()
        .ok_or(internal_detail("session key missing at issue"))?;
    let key = Secret::from_untrusted(raw);
    let view = crate::auth_csrf::credential_view(issue.state, issue.origin).await;
    let cookie = crate::auth_credential::cookie_issue(&view, &key, SESSION_COOKIE_MAX_AGE);
    let mut answer = answer;
    answer.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| internal_detail(&e.to_string()))?,
    );
    Ok(answer)
}

pub(crate) fn origin_of(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(String::from)
}
