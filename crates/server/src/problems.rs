use axum::Json;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

use anybot_messages::{Catalog, MessageCode, Params};
use anybot_protocol::{PROBLEM_JSON_CONTENT_TYPE, PROBLEM_TYPE_BASE, ProblemJson};

fn format_en(code: MessageCode, params: &Params) -> String {
    Catalog::new()
        .format_en(&code, params)
        .unwrap_or_else(|err| format!("{}: {err}", code.as_str()))
}

#[must_use]
pub fn auth_problem(status: u16, code: MessageCode) -> Box<ProblemJson> {
    auth_problem_params(status, code, &Params::default())
}

#[must_use]
pub fn auth_problem_params(status: u16, code: MessageCode, params: &Params) -> Box<ProblemJson> {
    let code_text = code.as_str();
    let title = format_en(code, params);
    Box::new(ProblemJson::new(
        status,
        title,
        format!("{PROBLEM_TYPE_BASE}/{}", code_text.replace('-', "_")),
        code_text.to_owned(),
    ))
}

#[must_use]
pub fn status_of(problem: &ProblemJson) -> StatusCode {
    StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn problem_response(problem: Box<ProblemJson>) -> Response {
    let status = status_of(&problem);
    (
        status,
        [(CONTENT_TYPE.as_str(), PROBLEM_JSON_CONTENT_TYPE)],
        Json(*problem),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_carries_the_code_and_suffix() {
        let problem = auth_problem(403, MessageCode::AccountInviteInvalid);
        assert_eq!(problem.code, "account-invite-invalid");
        assert_eq!(
            problem.problem_type,
            "https://anybot.test/problems/account_invite_invalid",
        );
        assert_eq!(problem.status, 403);
        assert_eq!(problem.title, "The invite code is wrong");
    }
}
