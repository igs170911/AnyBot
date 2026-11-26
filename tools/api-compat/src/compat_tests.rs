use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::compat;
use crate::schema::breaking;
use crate::sha::baseline;

#[must_use]
pub fn scratch_repo(name: &str) -> PathBuf {
    let test_name = std::thread::current().name().unwrap_or("test").to_owned();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(name)
        .join(test_name);
    let _removed = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join("api")).unwrap();
    git(&repo, &["init", "-b", "main"]);
    std::fs::write(
        repo.join(".gitignore"),
        "target/
",
    )
    .unwrap();
    git(
        &repo,
        &[
            "-c",
            "user.email=selftest@invalid",
            "-c",
            "user.name=selftest",
            "commit",
            "--allow-empty",
            "-m",
            "initial empty commit",
        ],
    );
    repo
}

pub fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

#[must_use]
pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/api-compat-selftest/fixtures")
            .join(name),
    )
    .unwrap()
}

pub fn commit(repo: &Path, text: &str) {
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

pub fn write_api(repo: &Path, openapi_text: &str, asyncapi_text: &str) {
    std::fs::write(repo.join("api/openapi.yaml"), openapi_text).unwrap();
    std::fs::write(repo.join("api/asyncapi.yaml"), asyncapi_text).unwrap();
}

#[must_use]
pub fn run(repo: &Path, git_ref: &str) -> Vec<String> {
    let inputs = compat::Inputs {
        repo,
        git_ref,
        read_ref: &compat::read_ref,
        tracked: &compat::changelog_tracked,
        release_tags: &compat::release_tags,
        breaking_of: &breaking,
    };
    compat::violations(&inputs)
}

#[must_use]
pub fn run_notes(repo: &Path, git_ref: &str) -> (Vec<String>, Vec<String>) {
    let inputs = compat::Inputs {
        repo,
        git_ref,
        read_ref: &compat::read_ref,
        tracked: &compat::changelog_tracked,
        release_tags: &compat::release_tags,
        breaking_of: &breaking,
    };
    compat::violations_with_notes(&inputs)
}

#[must_use]
pub fn seed(repo: &Path) -> String {
    write_api(
        repo,
        &fixture("openapi.yaml"),
        &fixture("asyncapi-base.yaml"),
    );
    let sha = baseline(
        fixture("openapi.yaml").as_bytes(),
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

pub fn touch_changelog(repo: &Path) {
    let mut text = std::fs::read_to_string(repo.join("api/breaking-changes.log")).unwrap();
    text.push_str(
        "# a trailing comment line: the files themselves do not change
",
    );
    std::fs::write(repo.join("api/breaking-changes.log"), text).unwrap();
    commit(repo, "touch the changelog comment");
}

pub fn append_record(repo: &Path, sha: &str, what: &str) {
    use std::fmt::Write as _;
    let mut text = std::fs::read_to_string(repo.join("api/breaking-changes.log")).unwrap();
    write!(
        text,
        "2026-01-02 | sha256={sha} | {what} | recorded for the gate test"
    )
    .unwrap();
    text.push('\n');
    std::fs::write(repo.join("api/breaking-changes.log"), text).unwrap();
}

#[test]
pub fn test_api_compat_selftest_blocks_an_untracked_record_file() {
    let repo = scratch_repo("api-compat-rust-untracked");
    let _seed_sha = seed(&repo);
    git(&repo, &["rm", "--cached", "api/breaking-changes.log"]);
    git(
        &repo,
        &[
            "-c",
            "user.email=selftest@invalid",
            "-c",
            "user.name=selftest",
            "commit",
            "-m",
            "drop the record from the index",
        ],
    );
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("not tracked by git")),
        "{found:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_blocks_breaking_changes_without_a_record() {
    let repo = scratch_repo("api-compat-rust-breaking");
    let _seed_sha = seed(&repo);
    write_api(
        &repo,
        &fixture("openapi.yaml"),
        &fixture("asyncapi-breaking.yaml"),
    );
    commit(&repo, "remove a payload property");
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_passes_recorded_breaking_changes_with_a_report() {
    let repo = scratch_repo("api-compat-rust-recorded-breaking");
    let _seed_sha = seed(&repo);
    write_api(
        &repo,
        &fixture("openapi.yaml"),
        &fixture("asyncapi-breaking.yaml"),
    );
    let sha = baseline(
        fixture("openapi.yaml").as_bytes(),
        fixture("asyncapi-breaking.yaml").as_bytes(),
    );
    append_record(&repo, &sha, "a payload property was removed");
    commit(&repo, "record the breaking change");
    let (found, notes) = run_notes(&repo, "HEAD");
    assert!(found.is_empty(), "violations {found:?} notes {notes:?}");
    assert!(
        notes
            .iter()
            .any(|line| line.contains("breaking protocol change")),
        "violations {found:?} notes {notes:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_blocks_baseline_changes_without_a_record() {
    let repo = scratch_repo("api-compat-rust-norecord");
    let _seed_sha = seed(&repo);
    write_api(
        &repo,
        &fixture("openapi.yaml"),
        &fixture("asyncapi-added.yaml"),
    );
    commit(&repo, "add a payload property");
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_blocks_openapi_changes_without_a_record() {
    let repo = scratch_repo("api-compat-rust-openapi");
    let _seed_sha = seed(&repo);
    let mut openapi = fixture("openapi.yaml");
    openapi.push_str("# an extra comment line changes the baseline hash\n");
    write_api(&repo, &openapi, &fixture("asyncapi-base.yaml"));
    commit(&repo, "touch the openapi file");
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_blocks_edits_of_older_record_lines() {
    let repo = scratch_repo("api-compat-rust-prefix");
    let _seed_sha = seed(&repo);
    write_api(
        &repo,
        &fixture("openapi.yaml"),
        &fixture("asyncapi-added.yaml"),
    );
    let sha = baseline(
        fixture("openapi.yaml").as_bytes(),
        fixture("asyncapi-added.yaml").as_bytes(),
    );
    let old = std::fs::read_to_string(repo.join("api/breaking-changes.log")).unwrap();
    let edited = old.replace("fixtures seeded", "seed renamed");
    let text = format!("{edited}2026-01-02 | sha256={sha} | a property was added | appended\n");
    std::fs::write(repo.join("api/breaking-changes.log"), text).unwrap();
    commit(&repo, "rewrite an old record line");
    let found = run(&repo, "HEAD");
    assert!(
        found.iter().any(|line| line.contains("prefix")),
        "{found:?}"
    );
}

#[test]
pub fn test_api_compat_selftest_blocks_a_tail_hash_that_does_not_match_the_baselines() {
    let repo = scratch_repo("api-compat-rust-tailhash");
    let _seed_sha = seed(&repo);
    append_record(&repo, &"0".repeat(64), "a property was added");
    commit(&repo, "record with the wrong hash");
    let found = run(&repo, "HEAD");
    assert!(
        found
            .iter()
            .any(|line| line.contains("regenerated baseline must record")),
        "{found:?}"
    );
}
