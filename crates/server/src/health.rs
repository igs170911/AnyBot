use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use anybot_protocol::{
    PROBLEM_CODE_INTERNAL, PROBLEM_TITLE_INTERNAL, PROBLEM_TYPE_BASE, PROTOCOL_VERSION, ProblemJson,
};

use crate::app::AppState;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "0.1.0")]
    pub version: String,
    #[schema(example = "2026-01-01T00:00:00Z")]
    pub now: String,
    #[schema(example = 1)]
    pub protocol_version: u32,
}

#[derive(Debug)]
pub struct HealthInternalError {
    problem: Box<ProblemJson>,
}

impl axum::response::IntoResponse for HealthInternalError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self.problem)).into_response()
    }
}

#[utoipa::path(
    get,
    path = "/v1/health",
    operation_id = "get_health",
    summary = "Server health check",
    description = "Returns the server version, the current time (RFC 3339) and the event protocol version; no identity required.",
    tag = "system",
    security(()),
    responses(
        (status = 200, description = "The server is alive",
            content_type = "application/json",
            body = HealthResponse,
            example = json!({"version": "0.1.0", "now": "2026-01-01T00:00:00Z", "protocol_version": 1})),
        (status = 400, description = "The request was malformed",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/bad-request", "title": "The request was malformed.", "status": 400, "code": "problem-bad-request", "params": {}})),
        (status = 500, description = "The server hit an internal error",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/internal", "title": "The server hit an internal error.", "status": 500, "code": "problem-internal", "params": {}})),
    )
)]
pub async fn health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, HealthInternalError> {
    let now = state
        .clock
        .now_ms()
        .rfc3339()
        .ok_or_else(internal_problem)?;
    Ok(Json(HealthResponse {
        version: state.version.clone(),
        now,
        protocol_version: PROTOCOL_VERSION,
    }))
}

fn internal_problem() -> HealthInternalError {
    let problem = Box::new(ProblemJson::new(
        500,
        PROBLEM_TITLE_INTERNAL.to_owned(),
        format!("{PROBLEM_TYPE_BASE}/internal"),
        PROBLEM_CODE_INTERNAL.to_owned(),
    ));
    HealthInternalError { problem }
}
