use super::*;
use std::path::PathBuf;

fn scratch_repo(tag: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(tag);
    assert!(std::fs::create_dir_all(repo.join(WORKFLOWS_REL)).is_ok());
    repo
}

fn write_workflow(repo: &Path, body: &str) {
    assert!(std::fs::write(repo.join(WORKFLOWS_REL).join("ci.yml"), body).is_ok());
}

const GOOD: &str = "name: CI\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8\n";

#[test]
fn test_workflows_gate_blocks_unpinned_uses() {
    let repo = scratch_repo("iron-workflows-unpinned");
    write_workflow(
        &repo,
        &GOOD.replace(
            "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8",
            "actions/checkout@v4",
        ),
    );
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(
        v.first()
            .is_some_and(|x| x.contains("pinned") && x.contains("actions/checkout@v4")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_workflows_gate_blocks_missing_permissions() {
    let repo = scratch_repo("iron-workflows-no-permissions");
    write_workflow(&repo, &GOOD.replace("permissions:\n  contents: read\n", ""));
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(
        v.first().is_some_and(|x| x.contains("permissions")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_workflows_gate_blocks_missing_timeout_minutes() {
    let repo = scratch_repo("iron-workflows-no-timeout");
    write_workflow(&repo, &GOOD.replace("    timeout-minutes: 30\n", ""));
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(
        v.first().is_some_and(|x| x.contains("timeout-minutes")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_workflows_gate_blocks_invalid_yaml() {
    let repo = scratch_repo("iron-workflows-bad-yaml");
    write_workflow(&repo, "name: CI\njobs:\n  check:\n\ttabs: not allowed\n");
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|x| x.contains("valid YAML")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_workflows_gate_accepts_local_action_paths() {
    assert!(use_ref_is_pinned("./actions/setup"));
    assert!(use_ref_is_pinned(
        "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8"
    ));
    assert!(!use_ref_is_pinned("actions/checkout@v4"));
    assert!(!use_ref_is_pinned("actions/checkout"));
    assert!(!use_ref_is_pinned(
        "actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8b"
    ));
    assert!(!use_ref_is_pinned("@v4"));
}

#[test]
fn test_workflows_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
}
