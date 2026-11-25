use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::fake_model::ModelFormat;
use crate::model_script::{ErrorBodyKind, Step, UsageScript};

#[must_use]
pub fn error_body(kind: ErrorBodyKind, status: u16, message: &str) -> String {
    match kind {
        ErrorBodyKind::OpenAi => json!({
            "error": {
                "message": message,
                "type": openai_type(status),
                "param": null,
                "code": openai_code(status, message),
            }
        })
        .to_string(),
        ErrorBodyKind::Anthropic => json!({
            "type": "error",
            "error": { "type": anthropic_type(status), "message": message }
        })
        .to_string(),
        ErrorBodyKind::Vllm => json!({
            "error": {
                "message": message,
                "type": "BadRequestError",
                "param": null,
                "code": status,
            }
        })
        .to_string(),
    }
}

fn openai_type(status: u16) -> &'static str {
    match status {
        402 => "insufficient_quota",
        429 => "rate_limit_error",
        500..=599 => "server_error",
        _ => "invalid_request_error",
    }
}

fn openai_code(status: u16, message: &str) -> serde_json::Value {
    if message.contains("maximum context length") {
        return json!("context_length_exceeded");
    }
    match status {
        402 => json!("insufficient_quota"),
        429 => json!("rate_limit_exceeded"),
        _ => serde_json::Value::Null,
    }
}

fn anthropic_type(status: u16) -> &'static str {
    match status {
        401 => "authentication_error",
        403 => "permission_error",
        404 => "not_found_error",
        429 => "rate_limit_error",
        529 => "overloaded_error",
        500..=599 => "api_error",
        _ => "invalid_request_error",
    }
}

#[must_use]
pub fn status_response(step: &Step) -> Option<Response> {
    match step {
        Step::ReplyStatus {
            status,
            message,
            retry_after_secs,
            body_kind,
        } => Some(coded(*status, message, *retry_after_secs, *body_kind)),
        Step::EmptyBody => Some(plain(StatusCode::OK, "")),
        Step::Redirect { location } => Some(redirect(location)),
        Step::ModelList { ids, silent } => Some(model_list(ids, *silent)),
        Step::ReplyText { .. }
        | Step::ThinkThenReply { .. }
        | Step::CallTool { .. }
        | Step::NoResponse { .. }
        | Step::CutOffAfter { .. }
        | Step::StallAfter { .. }
        | Step::StreamError { .. }
        | Step::UsageReply { .. }
        | Step::ThinkingBlock { .. }
        | Step::StopReason { .. }
        | Step::ToolCalls { .. }
        | Step::ThinkThenTool { .. } => None,
    }
}

fn coded(status: u16, message: &str, retry_after: Option<u64>, kind: ErrorBodyKind) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = error_body(kind, status, message);
    let mut response = (
        code,
        [(header::CONTENT_TYPE, "application/json")],
        Body::from(body),
    )
        .into_response();
    if let Some(secs) = retry_after {
        let value =
            HeaderValue::from_str(&secs.to_string()).unwrap_or(HeaderValue::from_static("0"));
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn plain(code: StatusCode, text: &str) -> Response {
    (code, Body::from(text.to_owned())).into_response()
}

fn redirect(location: &str) -> Response {
    let mut response = (StatusCode::FOUND, Body::from(String::new())).into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
}

fn model_list(ids: &[String], silent: bool) -> Response {
    if silent {
        return Response::default();
    }
    let data: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| json!({ "id": id, "object": "model", "owned_by": "fake" }))
        .collect();
    let body = json!({ "object": "list", "data": data }).to_string();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Body::from(body),
    )
        .into_response()
}

#[must_use]
pub fn openai_usage(usage: &UsageScript) -> serde_json::Value {
    json!({
        "prompt_tokens": usage.input_tokens,
        "completion_tokens": usage.output_tokens,
        "total_tokens": usage.input_tokens.saturating_add(usage.output_tokens),
        "prompt_tokens_details": { "cached_tokens": usage.cache_read_tokens },
        "completion_tokens_details": { "reasoning_tokens": usage.thinking_tokens },
    })
}

#[must_use]
pub fn anthropic_usage(usage: &UsageScript) -> serde_json::Value {
    json!({
        "input_tokens": usage.input_tokens,
        "cache_read_input_tokens": usage.cache_read_tokens,
        "cache_creation_input_tokens": usage.cache_write_tokens,
        "output_tokens": usage.output_tokens,
    })
}

#[must_use]
pub fn stream_error_event(format: ModelFormat, busy: bool, message: &str) -> String {
    let payload = match format {
        ModelFormat::Anthropic => json!({
            "type": "error",
            "error": {
                "type": if busy { "overloaded_error" } else { "api_error" },
                "message": message,
            }
        }),
        ModelFormat::OpenAi | ModelFormat::Vllm => json!({
            "error": {
                "message": message,
                "type": if busy { "server_error" } else { "invalid_request_error" },
                "code": if busy { "engine_busy" } else { "stream_error" },
            }
        }),
    };
    format!("event: error\ndata: {payload}\n\n")
}
