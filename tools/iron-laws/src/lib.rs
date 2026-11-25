#![forbid(unsafe_code)]

use serde as _;

pub mod banlists;
pub mod clippy_tomls;
pub mod collect;
pub mod comments;
pub mod compiled;
pub mod deny_limits;
pub mod editions;
pub mod exemptions;
pub mod licensing;
pub mod lines;
pub mod lints;
pub mod makefile_recipes;
pub mod makefile_scan;
pub mod nextest_config;
pub mod redocly;
pub mod suppressions;
pub mod table;
pub mod workflows;

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
    out.extend(lints::violations(repo));
    out.extend(compiled::violations_at_repo(repo));
    out.extend(licensing::violations(repo));
    out.extend(nextest_config::violations(repo));
    out.extend(redocly::violations(repo));
    out.extend(workflows::violations(repo));
    out.extend(makefile_recipes::violations(repo));
    out.sort();
    out
}

#[must_use]
pub fn run(repo: &Path) -> Vec<String> {
    all_violations(repo)
}
use serde as _;
