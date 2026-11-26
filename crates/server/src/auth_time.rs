use axum::extract::Request;
use axum::http::HeaderName;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;

pub const SERVER_TIME_HEADER: &str = "anybot-server-time";

pub async fn time_layer(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    request: Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;
    let stamp = HeaderValue::from_str(&state.clock.now_ms().millis().to_string());
    let Ok(stamp) = stamp else {
        return response;
    };
    let name = HeaderName::from_static(SERVER_TIME_HEADER);
    let mut response = response;
    response.headers_mut().insert(name, stamp);
    response
}
