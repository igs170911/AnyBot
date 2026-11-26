use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::compat::violations;
use crate::schema::breaking;
use crate::sha::baseline;
use crate::sha::is_hex64;

fn scratch_repo(name: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(name);
    let _removed = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join("api")).unwrap();
    git(&repo, &["init", "-b", "main"]);
    repo
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn openapi() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/api-compat-selftest/fixtures/openapi.yaml"),
    )
    .unwrap()
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/api-compat-selftest/fixtures")
            .join(name),
    )
    .unwrap()
}

fn commit(repo: &Path, text: &str) {
    git(repo, &["add", "-A"]);
    git(
        repo,
        &[
            "-c",
            "user.email=selftest@invalid",
            "-c",
            "user.name=selftest",
            "commit",
            "-m",
            text,
        ],
    );
}

fn write_api(repo: &Path, asyncapi_text: &str) {
    std::fs::write(repo.join("api/openapi.yaml"), openapi()).unwrap();
    std::fs::write(repo.join("api/asyncapi.yaml"), asyncapi_text).unwrap();
}

fn run(repo: &Path, git_ref: &str) -> Vec<String> {
    violations(&repo.join("."), git_ref, &baseline, &breaking)
}

fn seed(repo: &Path) -> String {
    write_api(repo, &fixture("asyncapi-base.yaml"));
    let sha = baseline(
        openapi().as_bytes(),
        fixture("asyncapi-base.yaml").as_bytes(),
    );
    std::fs::write(
        repo.join("api/breaking-changes.log"),
        format!("2026-01-01 | sha256={sha} | fixtures seeded | the selftest starts from a committed baseline\n"),
    )
    .unwrap();
    commit(repo, "seed");
    sha
}

#[test]
fn test_api_compat_selftest_blocks_breaking_protocol_changes() {
    let repo = scratch_repo("api-compat-rust-breaking");
    seed(&repo);
    write_api(&repo, &fixture("asyncapi-breaking.yaml"));
    let found = run(&repo, "HEAD");
    assert!(
        found
            .iter()
            .any(|line| line.contains("breaking protocol change")),
        "{found:?}"
    );
}

#[test]
fn test_api_compat_selftest_blocks_baseline_changes_without_a_record() {
    let repo = scratch_repo("api-compat-rust-norecord");
    seed(&repo);
    write_api(&repo, &fixture("asyncapi-added.yaml"));
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
}

#[test]
fn test_api_compat_selftest_blocks_malformed_changelog_lines() {
    let repo = scratch_repo("api-compat-rust-malformed");
    seed(&repo);
    std::fs::write(repo.join("api/breaking-changes.log"), "garbage line\n").unwrap();
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("not 64 hex digits")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|line| line.contains("YYYY-MM-DD")),
        "{found:?}"
    );
}

#[test]
fn test_api_compat_selftest_passes_recorded_additive_changes() {
    let repo = scratch_repo("api-compat-rust-recorded");
    seed(&repo);
    write_api(&repo, &fixture("asyncapi-added.yaml"));
    let sha = baseline(
        openapi().as_bytes(),
        fixture("asyncapi-added.yaml").as_bytes(),
    );
    assert!(is_hex64(&sha));
    std::fs::write(
        repo.join("api/breaking-changes.log"),
        format!(
            "2026-01-01 | sha256={} | fixtures seeded | seed\n2026-01-02 | sha256={sha} | a property was added | additive change\n",
            baseline(openapi().as_bytes(), fixture("asyncapi-base.yaml").as_bytes())
        ),
    )
    .unwrap();
    let found = run(&repo, "HEAD");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn test_api_compat_selftest_needs_committed_baselines_at_the_ref() {
    let repo = scratch_repo("api-compat-rust-noref");
    git(&repo, &["init"]);
    let found = run(&repo, "HEAD");
    assert!(!found.is_empty(), "{found:?}");
}
