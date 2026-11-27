#![forbid(unsafe_code)]

pub mod banlists;
pub mod cfg_test;
pub mod clippy_tomls;
pub mod collect;
pub mod comments;
pub mod compile_fail_names;
pub mod compiled;
pub mod deny_limits;
pub mod editions;
pub mod exemptions;
pub mod licensing;
pub mod lines;
pub mod lints;
pub mod makefile_recipes;
pub mod makefile_scan;
pub mod migrations;
pub mod migrations_committed;
#[cfg(test)]
mod migrations_committed_tests;
pub mod migrations_insert;
pub mod migrations_parser;
pub mod migrations_rules;
pub mod nextest_config;
pub mod passguard;
#[cfg(test)]
mod nextest_config_tests;
pub mod permit_block;
pub mod redocly;
pub mod suppressions;
pub mod table;
pub mod windows_lists;
pub mod workflow_needles;
pub mod workflow_runners;
pub mod workflows;

use std::path::Path;

#[must_use]
pub fn all_violations(repo: &Path, git_ref: &str) -> Vec<String> {
    let files = collect::source_files(repo);
    let mut out = suppressions::violations(&files);
    out.extend(comments::violations(&files));
    out.extend(cfg_test::violations(&files));
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
    out.extend(passguard::violations(&files));
    out.extend(compile_fail_names::violations(repo));
    out.extend(windows_lists::violations(repo));
    out.extend(redocly::violations(repo));
    out.extend(workflows::violations(repo));
    out.extend(makefile_recipes::violations(repo));
    out.extend(migrations::violations(repo));
    out.extend(migrations_committed::violations(repo, git_ref));
    out.sort();
    out
}

#[must_use]
pub fn run(repo: &Path, git_ref: &str) -> Vec<String> {
    all_violations(repo, git_ref)
}
