use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use tokio::net::TcpListener;

use crate::fake_clock::FakeClock;
use crate::model_plan;
use crate::model_script::{
    CODE_MODEL_SCRIPT_EMPTY, RecordedRequest, Script, Step, classify_key, presented_key,
    record_headers,
};

#[derive(Default)]
struct ModelInner {
    script: Script,
    recorded: Vec<RecordedRequest>,
    format: ModelFormat,
    expected_key: Option<String>,
    header_wait: Option<Arc<FakeClock>>,
}

#[derive(Default)]
pub struct FakeModel {
    inner: Mutex<ModelInner>,
}

impl FakeModel {
    pub fn script_set(&self, steps: Vec<Step>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.script.clear();
        for step in steps {
            inner.script.push(step);
        }
    }

    pub fn script_clear(&self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.script.clear();
    }

    pub fn script_len(&self) -> usize {
        let Ok(inner) = self.inner.lock() else {
            return 0;
        };
        inner.script.len()
    }

    pub fn format_set(&self, format: ModelFormat) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.format = format;
    }

    pub fn header_clock(&self, clock: Arc<FakeClock>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.header_wait = Some(clock);
    }

    pub fn expect_key(&self, key: &str) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.expected_key = Some(key.to_owned());
    }

    #[must_use]
    pub fn format(&self) -> ModelFormat {
        let Ok(inner) = self.inner.lock() else {
            return ModelFormat::OpenAi;
        };
        inner.format
    }

    pub fn recorded(&self) -> Vec<RecordedRequest> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.recorded.clone()
    }

    fn record(&self, path: &str, body: String, headers: &axum::http::HeaderMap) {
        let pairs: Vec<(String, String)> = headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let presented = pairs
            .iter()
            .find_map(|(name, value)| presented_key(name, value));
        let recorded = record_headers(pairs.into_iter());
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let key_match = classify_key(inner.expected_key.as_deref(), presented.as_deref());
        inner.recorded.push(RecordedRequest {
            path: path.to_owned(),
            body,
            headers: recorded,
            key_match,
        });
    }

    fn header_wait(&self) -> Option<Arc<FakeClock>> {
        let Ok(inner) = self.inner.lock() else {
            return None;
        };
        inner.header_wait.clone()
    }

    fn take_step(&self) -> Option<Step> {
        let Ok(mut inner) = self.inner.lock() else {
            return None;
        };
        inner.script.take()
    }
}

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
    let Some(step) = model.take_step() else {
        return empty_script_response();
    };
    model_plan::list_response(&step)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ModelFormat {
    #[default]
    OpenAi,
    Anthropic,
    Vllm,
}

#[must_use]
async fn respond(model: &Arc<FakeModel>, request_body: &str, format: ModelFormat) -> Response {
    let stream = wants_stream(request_body);
    let Some(step) = model.take_step() else {
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
        | Step::ThinkThenTool { .. }) => (other, None),
    }
}

fn wants_stream(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    value
        .get("stream")
        .is_some_and(|s| s.as_bool().unwrap_or(false))
}

fn requested_model(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return String::new();
    };
    value
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_owned()
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

#[must_use]
pub fn header_pairs(headers: &axum::http::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                value.to_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

pub fn find_header(headers: &axum::http::HeaderMap, name: HeaderName) -> Option<HeaderValue> {
    headers.get(name).cloned()
}
