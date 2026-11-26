use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use anybot_protocol::{PROTOCOL_VERSION, ProblemJson};

use crate::app::AppState;

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    #[schema(example = "0.1.0")]
    pub version: String,
    #[schema(example = 1)]
    pub protocol_version: u32,
    #[schema(example = 1200)]
    pub uptime_ms: u64,
    #[schema(example = 8787)]
    pub listen_port: u16,
    #[schema(example = "https://bot.example.test")]
    pub public_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/server/status",
    operation_id = "get_server_status",
    summary = "Public server status",
    description = "Returns version, protocol version, uptime, listen port and the configured public URL; no identity required.",
    tag = "system",
    security(()),
    responses(
        (status = 200, description = "Server status",
            content_type = "application/json",
            body = StatusResponse,
            example = json!({"version": "0.1.0", "protocol_version": 1, "uptime_ms": 1200, "listen_port": 8787, "public_url": null})),
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
pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let uptime_ms = state
        .clock
        .now_ms()
        .millis()
        .saturating_sub(state.started_at_ms.millis());
    Json(StatusResponse {
        version: state.version.clone(),
        protocol_version: PROTOCOL_VERSION,
        uptime_ms,
        listen_port: state.config.listen_port,
        public_url: state.config.public_url.clone(),
    })
}
