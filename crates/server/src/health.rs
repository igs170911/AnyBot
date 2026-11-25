use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use anybot_protocol::{PROBLEM_TYPE_BASE, PROTOCOL_VERSION, ProblemJson};

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

#[utoipa::path(
    get,
    path = "/v1/health",
    operation_id = "get_health",
    summary = "伺服器健康檢查",
    description = "回傳伺服器版本、現在時間（RFC 3339）和事件協定版本；不需要身分。",
    tag = "system",
    security(()),
    responses(
        (status = 200, description = "伺服器活著",
            content_type = "application/json",
            body = HealthResponse,
            example = json!({"version": "0.1.0", "now": "2026-01-01T00:00:00Z", "protocol_version": 1})),
        (status = 400, description = "請求格式錯誤",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/bad-request", "title": "請求格式錯誤", "status": 400})),
        (status = 500, description = "伺服器內部錯誤",
            content_type = "application/problem+json",
            body = ProblemJson,
            example = json!({"type": "https://anybot.test/problems/internal", "title": "伺服器內部錯誤", "status": 500})),
    )
)]
pub async fn health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, (StatusCode, Json<ProblemJson>)> {
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

fn internal_problem() -> (StatusCode, Json<ProblemJson>) {
    let problem = ProblemJson::new(
        500,
        "伺服器內部錯誤".to_owned(),
        format!("{PROBLEM_TYPE_BASE}/internal"),
    );
    (StatusCode::INTERNAL_SERVER_ERROR, Json(problem))
}
