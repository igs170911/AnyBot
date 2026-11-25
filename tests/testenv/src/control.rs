use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};

use test_harness::model_script::Step;
use test_harness::netctl::{net_cut, net_restore};

use crate::state::{TestEnvState, advance_ms};

#[derive(serde::Deserialize)]
pub struct AdvanceBody {
    pub ms: u64,
}

#[derive(serde::Serialize)]
pub struct AdvanceReply {
    pub woken: usize,
    pub now_ms: u64,
}

async fn status(State(state): State<Arc<TestEnvState>>) -> Json<crate::state::EnvPorts> {
    Json(state.ports.clone())
}

async fn set_script(
    State(state): State<Arc<TestEnvState>>,
    Json(steps): Json<Vec<Step>>,
) -> StatusCode {
    state.model.script_set(steps);
    StatusCode::ACCEPTED
}

async fn list_requests(
    State(state): State<Arc<TestEnvState>>,
) -> Json<Vec<test_harness::model_script::RecordedRequest>> {
    Json(state.model.recorded())
}

async fn advance(
    State(state): State<Arc<TestEnvState>>,
    Json(body): Json<AdvanceBody>,
) -> Json<AdvanceReply> {
    let woken = advance_ms(&state, body.ms);
    Json(AdvanceReply {
        woken,
        now_ms: state.clock.now_ms(),
    })
}

async fn cut(State(state): State<Arc<TestEnvState>>) -> Result<StatusCode, String> {
    net_cut(&state.net, state.ports.proxy_port)
        .await
        .map(|()| StatusCode::ACCEPTED)
}

async fn restore(State(state): State<Arc<TestEnvState>>) -> Result<StatusCode, String> {
    net_restore(&state.net, state.ports.proxy_port)
        .await
        .map(|()| StatusCode::ACCEPTED)
}

async fn push_records(
    State(state): State<Arc<TestEnvState>>,
) -> Json<Vec<test_harness::fake_push::PushRecord>> {
    Json(state.push.records())
}

#[must_use = "the caller must serve this router"]
pub fn control_router(state: Arc<TestEnvState>) -> Router {
    Router::new()
        .route("/control/status", get(status))
        .route("/control/model/script", post(set_script))
        .route("/control/model/requests", get(list_requests))
        .route("/control/clock/advance", post(advance))
        .route("/control/net/cut", post(cut))
        .route("/control/net/restore", post(restore))
        .route("/control/push/records", get(push_records))
        .with_state(state)
}
