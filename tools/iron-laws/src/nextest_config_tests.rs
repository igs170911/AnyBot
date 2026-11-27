use std::path::PathBuf;

use super::nextest_config::violations;

fn scratch_repo(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch");
    let repo = root.join(name);
    std::fs::create_dir_all(repo.join(".config")).unwrap();
    repo
}

const QUICK: &str =
    "[profile.quick]\ndefault-filter = 'not (binary(/^pg_/) | test(/_compile_fail$/))'\n";

fn with_quick(head: &str) -> String {
    format!("{head}{QUICK}")
}

#[test]
fn test_nextest_config_gate_blocks_missing_file() {
    let repo = scratch_repo("iron-nextest-missing");
    std::fs::remove_dir_all(repo.join(".config")).unwrap();
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|x| x.contains("missing")), "{v:?}");
}

#[test]
fn test_nextest_config_gate_blocks_missing_terminate_after() {
    let repo = scratch_repo("iron-nextest-no-kill");
    std::fs::write(
        repo.join(".config").join("nextest.toml"),
        "[profile.default]\nslow-timeout = { period = \"30s\" }\n",
    )
    .unwrap();
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(
        v.first().is_some_and(|x| x.contains("terminate-after")),
        "{v:?}"
    );
}

#[test]
fn test_nextest_config_gate_blocks_serial_key() {
    let repo = scratch_repo("iron-nextest-serial");
    std::fs::write(
        repo.join(".config").join("nextest.toml"),
        with_quick("[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n\n[[profile.default.serial]]\nparallelism = 1\n"),
    )
    .unwrap();
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|x| x.contains("serial")), "{v:?}");
}

#[test]
fn test_nextest_config_gate_blocks_group_key_in_overrides() {
    let repo = scratch_repo("iron-nextest-group-key");
    std::fs::write(
        repo.join(".config").join("nextest.toml"),
        with_quick("[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n\n[[profile.default.overrides]]\nfilter = 'test(x)'\ngroup = \"pg\"\n"),
    )
    .unwrap();
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|x| x.contains("test-group")), "{v:?}");
}

#[test]
fn test_nextest_config_gate_blocks_profile_test_groups_table() {
    let repo = scratch_repo("iron-nextest-profile-groups");
    std::fs::write(
        repo.join(".config").join("nextest.toml"),
        with_quick("[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n[profile.default.test-groups]\nparallelism = 1\n"),
    )
    .unwrap();
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(
        v.first().is_some_and(|x| x.contains("[test-groups]")),
        "{v:?}"
    );
}

#[test]
fn test_nextest_config_gate_blocks_pg_start_outside_pg_file() {
    let repo = scratch_repo("iron-nextest-pg-name");
    std::fs::write(
        repo.join(".config").join("nextest.toml"),
        with_quick("[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n"),
    )
    .unwrap();
    let dir = repo.join("tests").join("harness").join("tests");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("socket.rs"), "fn t() { pg_start(&base); }\n").unwrap();
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("socket.rs") && x.contains("pg_start")),
        "{v:?}"
    );
}

#[test]
fn test_nextest_config_gate_accepts_real_config() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
}
