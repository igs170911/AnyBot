use std::path::{Component, Path, PathBuf};

#[must_use]
pub fn safe_relative(path: &str) -> Option<PathBuf> {
    let relative = Path::new(path.trim_start_matches('/'));
    let clean = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    clean.then(|| relative.to_path_buf())
}

#[must_use]
pub fn read_file(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

#[must_use]
pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("json" | "map") => "application/json",
        Some("webmanifest") => "application/manifest+json",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
