use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::fake_model::ModelFormat;
use crate::model_anthropic;
use crate::model_extra;
use crate::model_openai;
use crate::model_script::{CODE_MODEL_SCRIPT_EMPTY, Step};
use crate::model_vllm;

#[must_use]
pub fn stream_response(step: Step, format: ModelFormat, model: String) -> Response {
    if matches!(step, Step::ReplyStatus { .. }) {
        return json_response(step, format, &model);
    }
    let (tx, rx) = mpsc::channel::<String>(32);
    let plan = Plan {
        step,
        format,
        model,
    };
    tokio::spawn(run_plan(tx, plan));
    let streamed = ReceiverStream::new(rx);
    let lines = streamed.map(|line| Ok::<bytes::Bytes, std::io::Error>(bytes::Bytes::from(line)));
    let content_type = content_type_for(format);
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, content_type)],
        Body::from_stream(lines),
    )
        .into_response()
}

#[derive(Clone)]
pub struct Plan {
    pub step: Step,
    pub format: ModelFormat,
    pub model: String,
}

pub async fn run_plan(tx: mpsc::Sender<String>, plan: Plan) {
    if let Some(event) = stream_special(&plan) {
        let _sent = tx.send(event).await.is_ok();
        drop(tx);
        return;
    }
    match plan.format {
        ModelFormat::OpenAi => model_openai::plan_lines(&tx, plan.step, &plan.model).await,
        ModelFormat::Vllm => model_vllm::plan_lines(&tx, plan.step, &plan.model).await,
        ModelFormat::Anthropic => model_anthropic::plan_lines(&tx, plan.step, &plan.model).await,
    }
    drop(tx);
}

fn stream_special(plan: &Plan) -> Option<String> {
    match &plan.step {
        Step::StreamError { busy, message } => {
            Some(model_extra::stream_error_event(plan.format, *busy, message))
        }
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::ReplyStatus { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => None,
    }
}

#[must_use]
pub fn list_response(step: &Step) -> Response {
    model_extra::status_response(step).unwrap_or_else(empty_list)
}

fn empty_list() -> Response {
    (
        StatusCode::BAD_REQUEST,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        Body::from(empty_script_body()),
    )
        .into_response()
}

#[must_use]
pub fn json_response(step: Step, format: ModelFormat, model: &str) -> Response {
    if let Some(special) = model_extra::status_response(&step) {
        return special;
    }
    match step {
        Step::ReplyStatus { .. }
        | Step::EmptyBody
        | Step::Redirect { .. }
        | Step::ModelList { .. }
        | Step::DelayHeaders { .. }
        | Step::AcceptThenDelay { .. } => empty_list(),
        other @ (Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::HoldThenFinish { .. }
        | Step::StreamError { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. }) => {
            let body = match format {
                ModelFormat::OpenAi => model_openai::json_body(&other, model),
                ModelFormat::Vllm => model_vllm::json_body(&other, model),
                ModelFormat::Anthropic => model_anthropic::json_body(&other, model),
            };
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                Body::from(body),
            )
                .into_response()
        }
    }
}

#[must_use]
pub fn status_response(status: u16, message: &str) -> Response {
    model_extra::status_response(&Step::ReplyStatus {
        status,
        message: message.to_owned(),
        retry_after_secs: None,
        body_kind: crate::model_script::ErrorBodyKind::OpenAi,
    })
    .unwrap_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Body::from(String::new())).into_response()
    })
}

#[must_use]
pub fn empty_script_body() -> String {
    serde_json::json!({ "error": { "message": CODE_MODEL_SCRIPT_EMPTY } }).to_string()
}

#[must_use]
pub fn content_type_for(format: ModelFormat) -> &'static str {
    match format {
        ModelFormat::OpenAi | ModelFormat::Anthropic | ModelFormat::Vllm => "text/event-stream",
    }
}

#[must_use]
pub fn sse_line(payload: &str) -> String {
    format!("data: {payload}\n\n")
}

#[must_use]
pub fn json_line(value: &serde_json::Value) -> String {
    sse_line(&value.to_string())
}
