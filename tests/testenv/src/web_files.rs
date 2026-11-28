use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

use test_harness::scratch::repo_root;
use test_harness::static_files::{content_type, read_file, safe_relative};

fn dist_dir() -> PathBuf {
    repo_root().join("apps").join("web").join("dist")
}

fn file_response(path: &Path, bytes: Vec<u8>) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type(path))],
        Body::from(bytes),
    )
        .into_response()
}

pub async fn serve_web(uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/v1/") || path.starts_with("/control/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(relative) = safe_relative(path) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let wanted = if relative.as_os_str().is_empty() {
        PathBuf::from("index.html")
    } else {
        relative
    };
    let full = dist_dir().join(&wanted);
    if let Some(bytes) = read_file(&full) {
        return file_response(&full, bytes);
    }
    let index = dist_dir().join("index.html");
    match read_file(&index) {
        Some(bytes) if wanted.extension().is_none() => file_response(&index, bytes),
        _other => StatusCode::NOT_FOUND.into_response(),
    }
}
