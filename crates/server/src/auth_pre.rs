use axum::extract::Request;
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;

const PRE_SESSION_PATHS: [&str; 2] = ["/v1/account/login", "/v1/account/register"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreSessionMarker;

pub async fn pre_session_layer(request: Request, next: Next) -> Response {
    if is_pre_session(&request) {
        let (mut parts, body) = request.into_parts();
        parts.extensions.insert(PreSessionMarker);
        return next.run(Request::from_parts(parts, body)).await;
    }
    next.run(request).await
}

fn is_pre_session(request: &Request) -> bool {
    request.method() == Method::POST && PRE_SESSION_PATHS.contains(&request.uri().path())
}
