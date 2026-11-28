use axum::body::{Body, HttpBody};
use axum::extract::{FromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use http_body_util::Limited;

use anybot_messages::{MessageCode, Params};

use crate::app::AppState;
use crate::caller::Caller;
use crate::problems::{auth_problem_params, problem_response};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyCap {
    Guest,
    SignedIn,
}

impl BodyCap {
    #[must_use]
    pub const fn kilobytes(self) -> u32 {
        match self {
            BodyCap::Guest => 256,
            BodyCap::SignedIn => 64 * 1024,
        }
    }

    #[must_use]
    pub const fn bytes(self) -> usize {
        match self {
            BodyCap::Guest => 256 * 1024,
            BodyCap::SignedIn => 64 * 1024 * 1024,
        }
    }

    fn admits(self, declared: Option<u64>) -> bool {
        declared.is_none_or(|size| usize::try_from(size).is_ok_and(|size| size <= self.bytes()))
    }
}

async fn cap_for(parts: &mut Parts, state: &AppState) -> BodyCap {
    match Caller::from_request_parts(parts, state).await {
        Ok(_caller) => BodyCap::SignedIn,
        Err(_denied) => BodyCap::Guest,
    }
}

fn too_large(cap: BodyCap) -> Response {
    let params = Params::new().with_int("limit_kb", i64::from(cap.kilobytes()));
    problem_response(auth_problem_params(
        StatusCode::PAYLOAD_TOO_LARGE.as_u16(),
        MessageCode::ServerRequestTooLarge,
        &params,
    ))
}

pub async fn body_limit_layer(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let declared = request.body().size_hint().exact();
    if declared.is_some() && BodyCap::Guest.admits(declared) {
        return next.run(request).await;
    }
    let (mut parts, body) = request.into_parts();
    let cap = cap_for(&mut parts, &state).await;
    if !cap.admits(declared) {
        return too_large(cap);
    }
    let bounded = Body::new(Limited::new(body, cap.bytes()));
    next.run(Request::from_parts(parts, bounded)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_match_the_spec_numbers() {
        assert_eq!(BodyCap::Guest.bytes(), 262_144);
        assert_eq!(BodyCap::SignedIn.bytes(), 67_108_864);
        assert!(BodyCap::Guest.admits(Some(262_144)));
        assert!(!BodyCap::Guest.admits(Some(262_145)));
        assert!(BodyCap::SignedIn.admits(Some(67_108_864)));
        assert!(!BodyCap::SignedIn.admits(Some(67_108_865)));
        assert!(BodyCap::Guest.admits(None));
    }
}
