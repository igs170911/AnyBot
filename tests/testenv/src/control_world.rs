use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{delete, get, post};

use test_harness::model_script::{RecordedRequest, Step};

use crate::locks::{LOCK_WAIT, LockTable};
use crate::seed::{SeedReply, SeedRequest, disable_account, seed_world};
use crate::state::TestEnvState;

type Shared = Arc<TestEnvState>;
type Failure = (StatusCode, String);

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum ScriptBody {
    Named { model: String, steps: Vec<Step> },
    Shared(Vec<Step>),
}

#[derive(serde::Deserialize)]
pub struct RequestsQuery {
    pub model: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct LockBody {
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct DisableBody {
    pub account: String,
}

fn failed(message: String) -> Failure {
    (StatusCode::INTERNAL_SERVER_ERROR, message)
}

async fn set_script(State(state): State<Shared>, Json(body): Json<ScriptBody>) -> StatusCode {
    match body {
        ScriptBody::Named { model, steps } => state.model.script_set_for(&model, steps),
        ScriptBody::Shared(steps) => state.model.script_set(steps),
    }
    StatusCode::ACCEPTED
}

async fn list_requests(
    State(state): State<Shared>,
    Query(query): Query<RequestsQuery>,
) -> Json<Vec<RecordedRequest>> {
    Json(match query.model {
        Some(model) => state.model.recorded_for(&model),
        None => state.model.recorded(),
    })
}

async fn seed(
    State(state): State<Shared>,
    Json(request): Json<SeedRequest>,
) -> Result<Json<SeedReply>, Failure> {
    let now_ms = i64::try_from(state.clock.now_ms()).map_err(|e| failed(e.to_string()))?;
    seed_world(&state.api, &state.admin, &request, now_ms)
        .await
        .map(Json)
        .map_err(failed)
}

async fn disable(
    State(state): State<Shared>,
    Json(body): Json<DisableBody>,
) -> Result<StatusCode, Failure> {
    disable_account(&state.api, &state.admin, &body.account)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(failed)
}

async fn take_lock(
    State(state): State<Shared>,
    Json(body): Json<LockBody>,
) -> Result<StatusCode, Failure> {
    let table: &LockTable = &state.locks;
    table
        .acquire(&body.name, LOCK_WAIT)
        .await
        .map(|()| StatusCode::OK)
        .map_err(|message| (StatusCode::REQUEST_TIMEOUT, message))
}

async fn drop_lock(State(state): State<Shared>, Path(name): Path<String>) -> StatusCode {
    if state.locks.release(&name) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/control/model/script", post(set_script))
        .route("/control/model/requests", get(list_requests))
        .route("/control/seed", post(seed))
        .route("/control/account/disable", post(disable))
        .route("/control/lock", post(take_lock))
        .route("/control/lock/{name}", delete(drop_lock))
}
