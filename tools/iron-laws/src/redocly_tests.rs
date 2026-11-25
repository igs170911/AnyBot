use super::*;
use std::path::PathBuf;

fn scratch_dir(tag: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(tag);
    assert!(std::fs::create_dir_all(&repo).is_ok());
    repo
}

#[test]
fn test_redocly_lint_ignore_file_is_blocked() {
    let repo = scratch_dir("iron-redocly-ignore");
    assert!(
        std::fs::write(
            repo.join(".redocly.lint-ignore.yaml"),
            "rules:\n  rule/x: []\n"
        )
        .is_ok()
    );
    let v = violations(&repo);
    assert!(
        v.first()
            .is_some_and(|x| x.contains(".redocly.lint-ignore.yaml") && x.contains("ignore file"))
    );
    assert!(std::fs::remove_file(repo.join(".redocly.lint-ignore.yaml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_redocly_off_and_warn_severities_are_blocked() {
    let repo = scratch_dir("iron-redocly-severity");
    let cfg = "rules:\n  rule/operation-has-5xx: off\n  anybot/idempotencyKeyRequired: warn\n  rule/no-empty-servers: error\n";
    assert!(std::fs::write(repo.join("redocly.yaml"), cfg).is_ok());
    let v = violations(&repo);
    assert_eq!(v.len(), 2);
    assert!(v.iter().all(|x| x.contains("redocly.yaml")));
    assert!(v.iter().any(|x| x.contains("off")));
    assert!(v.iter().any(|x| x.contains("warn")));
    assert!(std::fs::remove_file(repo.join("redocly.yaml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_redocly_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(violations(&repo).is_empty());
}
