use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use tokio::net::TcpListener;

use crate::model_plan;
use crate::model_request::{requested_model, wants_stream};
use crate::model_script::{CODE_MODEL_SCRIPT_EMPTY, Step};

use super::{FakeModel, ModelFormat};

pub async fn model_start() -> Result<(Arc<FakeModel>, u16), String> {
    model_start_with_format(ModelFormat::OpenAi).await
}

pub async fn model_start_with_format(format: ModelFormat) -> Result<(Arc<FakeModel>, u16), String> {
    let model = Arc::new(FakeModel::default());
    model.format_set(format);
    let (std_listener, port) = crate::ports::bind_v4()?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("set nonblocking failed: {e}"))?;
    let listener =
        TcpListener::from_std(std_listener).map_err(|e| format!("convert listener failed: {e}"))?;
    let router = model_router(Arc::clone(&model));
    tokio::spawn(async move {
        let _served = axum::serve(listener, router).await;
    });
    Ok((model, port))
}

pub fn model_router(model: Arc<FakeModel>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handle_openai))
        .route("/v1/messages", post(handle_anthropic))
        .route("/v1/models", axum::routing::get(handle_models))
        .route("/models", axum::routing::get(handle_models))
        .with_state(model)
}

async fn handle_openai(
    State(model): State<Arc<FakeModel>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let _accepted = tokio::task::yield_now().await;
    model.record("/v1/chat/completions", body.clone(), &headers);
    let format = match model.format() {
        ModelFormat::OpenAi | ModelFormat::Anthropic => ModelFormat::OpenAi,
        ModelFormat::Vllm => ModelFormat::Vllm,
    };
    respond(&model, &body, format).await
}

async fn handle_anthropic(
    State(model): State<Arc<FakeModel>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    model.record("/v1/messages", body.clone(), &headers);
    respond(&model, &body, ModelFormat::Anthropic).await
}

async fn handle_models(
    State(model): State<Arc<FakeModel>>,
    headers: axum::http::HeaderMap,
) -> Response {
    model.record("/v1/models", String::new(), &headers);
    let Some(step) = model.take_step("") else {
        return empty_script_response();
    };
    let (step, delay) = peel_delay(step);
    if let Some(seconds) = delay
        && let Some(clock) = model.header_wait()
    {
        tokio::task::yield_now().await;
        anybot_foundation::clock::Clock::sleep(
            clock.as_ref(),
            std::time::Duration::from_secs(seconds),
        )
        .await;
    }
    model_plan::list_response(&step)
}
#[must_use]
async fn respond(model: &Arc<FakeModel>, request_body: &str, format: ModelFormat) -> Response {
    let stream = wants_stream(request_body);
    let Some(step) = model.take_step(&requested_model(request_body)) else {
        return empty_script_response();
    };
    let (step, delay) = peel_delay(step);
    if let Some(seconds) = delay
        && let Some(clock) = model.header_wait()
    {
        tokio::task::yield_now().await;
        anybot_foundation::clock::Clock::sleep(
            clock.as_ref(),
            std::time::Duration::from_secs(seconds),
        )
        .await;
    }
    if stream {
        return model_plan::stream_response(step, format, requested_model(request_body));
    }
    model_plan::json_response(step, format, &requested_model(request_body))
}

fn peel_delay(step: Step) -> (Step, Option<u64>) {
    match step {
        Step::DelayHeaders { seconds, inner } | Step::AcceptThenDelay { seconds, inner } => {
            (*inner, Some(seconds))
        }
        other @ (Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::ReplyStatus { .. }
        | Step::StreamError { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. }
        | Step::ToolCallsWithUsage { .. }) => (other, None),
    }
}

fn empty_script_response() -> Response {
    let body = serde_json::json!({ "error": { "message": CODE_MODEL_SCRIPT_EMPTY } });
    let text = body.to_string();
    (
        StatusCode::BAD_REQUEST,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        Body::from(text),
    )
        .into_response()
}
