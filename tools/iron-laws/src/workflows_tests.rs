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
fn test_workflows_gate_blocks_top_level_write_permission() {
    let repo = scratch_repo("iron-workflows-write");
    write_workflow(
        &repo,
        &GOOD.replace(
            "permissions:\n  contents: read\n",
            "permissions:\n  contents: write\n",
        ),
    );
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("contents: read") && x.contains("write")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_allows_push_only_write_job_beside_pull_request() {
    let repo = scratch_repo("iron-workflows-push-write");
    let body = "name: CI\non:\n  push:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  publish:\n    if: github.event_name == 'push' && github.ref == 'refs/heads/dev'\n    permissions:\n      contents: write\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_or_pull_request_write_job() {
    let repo = scratch_repo("iron-workflows-or-write");
    let body = "name: CI\non:\n  push:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  publish:\n    if: github.event_name == 'push' || github.event_name == 'pull_request'\n    permissions:\n      contents: write\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("pull_request") && x.contains("write")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_write_job_on_pull_request() {
    let repo = scratch_repo("iron-workflows-pr-write");
    let body = "name: CI\non:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  publish:\n    permissions:\n      contents: write\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("pull_request") && x.contains("write")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_reads_pinned_matrix_runners() {
    let repo = scratch_repo("iron-workflows-matrix-pinned");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  desktop:\n    strategy:\n      matrix:\n        include:\n          - runner: ubuntu-24.04\n    runs-on: ${{ matrix.runner }}\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_latest_in_matrix_include() {
    let repo = scratch_repo("iron-workflows-matrix-latest");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  desktop:\n    strategy:\n      matrix:\n        include:\n          - runner: ubuntu-latest\n    runs-on: ${{ matrix.runner }}\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("ubuntu-latest") && x.contains("matrix")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_allows_caller_job_without_timeout() {
    let repo = scratch_repo("iron-workflows-caller-ok");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  release-dev:\n    needs: [check]\n    if: github.event_name == 'push'\n    uses: ./.github/workflows/release.yml\n    permissions:\n      contents: write\n    secrets: inherit\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_timeout_on_caller_job() {
    let repo = scratch_repo("iron-workflows-caller-timeout");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  release-dev:\n    uses: ./.github/workflows/release.yml\n    timeout-minutes: 180\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("timeout-minutes") && x.contains("calls a workflow")),
        "{v:?}"
    );
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_latest_runner() {
    let repo = scratch_repo("iron-workflows-latest");
    write_workflow(&repo, &GOOD.replace("ubuntu-24.04", "ubuntu-latest"));
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("latest")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_event_text_in_run_script() {
    let repo = scratch_repo("iron-workflows-injection");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ${{ github.event.pull_request.title }}\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("title")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_branch_name_in_run_script() {
    let repo = scratch_repo("iron-workflows-branch");
    let body = "name: CI\non:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo \"${{ github.head_ref }}\"\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("branch name")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_accepts_event_text_passed_through_env() {
    let repo = scratch_repo("iron-workflows-env");
    let body = "name: CI\non:\n  push:\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - env:\n          TITLE: ${{ github.event.pull_request.title }}\n        run: echo \"$TITLE\"\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_write_all() {
    let repo = scratch_repo("iron-workflows-write-all");
    let body = "name: CI\non:\n  pull_request:\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    permissions: write-all\n    steps:\n      - run: echo ok\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("write")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_blocks_release_tag_name_in_run() {
    let repo = scratch_repo("iron-workflows-tag-name");
    let body = "name: CI\non:\n  release:\npermissions:\n  contents: read\njobs:\n  check:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 30\n    steps:\n      - run: echo ${{ github.event.release.tag_name }}\n";
    write_workflow(&repo, body);
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("title")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(WORKFLOWS_REL).join("ci.yml")).is_ok());
}

#[test]
fn test_workflows_gate_fails_when_workflows_dir_is_missing() {
    let repo = scratch_repo("iron-workflows-missing-dir");
    std::fs::remove_dir_all(repo.join(WORKFLOWS_REL)).unwrap();
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("missing")), "{v:?}");
}

#[test]
fn test_workflows_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
}
