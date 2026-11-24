#![forbid(unsafe_code)]

pub mod clippy_tomls;
pub mod collect;
pub mod comments;
pub mod deny_limits;
pub mod lines;
pub mod suppressions;
pub mod table;

use std::path::Path;

#[must_use]
pub fn all_violations(repo: &Path) -> Vec<String> {
    let files = collect::source_files(repo);
    let mut out = suppressions::violations(&files);
    out.extend(comments::violations(&files));
    out.extend(lines::violations(&files));
    match collect::read_file_lossy(&repo.join("deny.toml")) {
        Ok(content) => out.extend(deny_limits::violations(&content)),
        Err(e) => out.push(e),
    }
    out.extend(clippy_tomls::violations(
        &clippy_tomls::root_config(repo),
        &collect::clippy_toml_files(repo),
    ));
    out.sort();
    out
}

#[must_use]
pub fn run(repo: &Path) -> Vec<String> {
    all_violations(repo)
}
