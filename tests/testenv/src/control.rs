use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};

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

#[derive(serde::Serialize)]
pub struct StatusView {
    pub ports: crate::state::EnvPorts,
    pub now_ms: u64,
}

async fn status(State(state): State<Arc<TestEnvState>>) -> Json<StatusView> {
    Json(StatusView {
        ports: state.ports.clone(),
        now_ms: state.clock.now_ms(),
    })
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
        .route("/control/clock/advance", post(advance))
        .route("/control/net/cut", post(cut))
        .route("/control/net/restore", post(restore))
        .route("/control/push/records", get(push_records))
        .merge(crate::control_world::routes())
        .with_state(state)
}
