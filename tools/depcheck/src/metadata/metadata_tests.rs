use crate::metadata::{Dependency, Metadata, Package, check_metadata_value};

fn pkg(name: &str, deps: &[&str]) -> Package {
    Package {
        name: name.to_owned(),
        source: None,
        dependencies: deps
            .iter()
            .map(|d| Dependency {
                name: (*d).to_owned(),
                kind: None,
            })
            .collect(),
    }
}

fn meta(packages: Vec<Package>) -> Metadata {
    Metadata { packages }
}

#[test]
fn test_depcheck_ok_for_allowed_edges() {
    let m = meta(vec![
        pkg("anybot-store-server", &["anybot-foundation"]),
        pkg("anybot-store-client", &["anybot-foundation"]),
        pkg(
            "anybot-server",
            &["anybot-store-server", "anybot-foundation"],
        ),
        pkg("anybot-sync", &["anybot-store-client"]),
    ]);
    assert_eq!(check_metadata_value(&m), Vec::<String>::new());
}

#[test]
fn test_depcheck_blocks_reverse_and_banned_edges() {
    let m = meta(vec![
        pkg("anybot-foundation", &["anybot-protocol"]),
        pkg("anybot-domain", &["tokio"]),
        pkg("anybot-server", &["anybot-runner"]),
        pkg("anybot-protocol", &[]),
        pkg("anybot-runner", &[]),
    ]);
    let v = check_metadata_value(&m);
    assert_eq!(v.len(), 3);
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-foundation depends on anybot-protocol"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-server depends on anybot-runner"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-domain depends on tokio"))
    );
}

#[test]
fn test_depcheck_allows_testenv_normal_harness_dep() {
    let m = meta(vec![
        pkg(
            "testenv",
            &["test-harness", "anybot-foundation", "anybot-server"],
        ),
        pkg("test-harness", &["serde"]),
        pkg("anybot-server", &[]),
        pkg("anybot-foundation", &[]),
    ]);
    assert_eq!(check_metadata_value(&m), Vec::<String>::new());
}

#[test]
fn test_depcheck_blocks_harness_in_normal_deps_and_reverse() {
    let m = meta(vec![
        pkg("anybot-server", &["test-harness"]),
        pkg("anybot-foundation", &[]),
        pkg("anybot-protocol", &[]),
        pkg("test-harness", &["anybot-protocol"]),
        pkg("test-harness", &["serde"]),
    ]);
    let v = check_metadata_value(&m);
    assert_eq!(v.len(), 3);
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-server uses test-harness in normal dependencies"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("test-harness depends on anybot-protocol"))
    );
}

#[test]
fn test_depcheck_blocks_unknown_package() {
    let m = meta(vec![pkg("anybot-mystery", &["anybot-foundation"])]);
    assert!(
        check_metadata_value(&m)
            .first()
            .is_some_and(|x| x.contains("anybot-mystery"))
    );
}

#[test]
fn test_depcheck_app_dependency_table() {
    let m = meta(vec![
        pkg("anybot-app-server", &["anybot-runner", "anybot-server"]),
        pkg(
            "anybot-app-runner-service",
            &["anybot-server", "anybot-store-server", "anybot-protocol"],
        ),
        pkg("anybot-app-agent", &["anybot-sync"]),
        pkg("anybot-app-cli", &["anybot-store-server"]),
    ]);
    let v = check_metadata_value(&m);
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-app-server depends on anybot-runner"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-app-runner-service depends on anybot-server"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-app-runner-service depends on anybot-store-server"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("anybot-app-cli depends on anybot-store-server"))
    );
    assert!(!v.iter().any(|s| s.contains("anybot-protocol")));
    assert!(!v.iter().any(|s| s.contains("anybot-sync")));
    assert!(
        !v.iter()
            .any(|s| s.contains("anybot-app-server depends on anybot-server"))
    );
}

#[test]
fn test_depcheck_desktop_shell_only_a_window() {
    let m = meta(vec![pkg(
        "anybot-app-desktop",
        &[
            "anybot-server",
            "anybot-store-server",
            "anybot-runner",
            "anybot-models",
            "anybot-tools",
            "anybot-computers",
            "anybot-mcp",
            "anybot-external-agents",
            "anybot-foundation",
        ],
    )]);
    let v = check_metadata_value(&m);
    assert_eq!(v.len(), 8);
    assert!(v.iter().all(|s| s.contains("anybot-app-desktop")));
}

#[test]
fn test_depcheck_allows_harness_to_depend_on_foundation() {
    let m = meta(vec![pkg("test-harness", &["anybot-foundation"])]);
    assert_eq!(check_metadata_value(&m), Vec::<String>::new());
}

#[test]
fn test_depcheck_blocks_runner_test_dependence_on_product_crates() {
    let m = meta(vec![
        pkg(
            "runner-test",
            &[
                "test-harness",
                "anybot-foundation",
                "anybot-protocol",
                "anybot-trust",
                "anybot-runner",
            ],
        ),
        pkg("anybot-runner", &["anybot-foundation"]),
        pkg("anybot-trust", &["anybot-foundation"]),
        pkg("anybot-protocol", &["anybot-foundation"]),
        pkg("anybot-foundation", &[]),
        pkg("test-harness", &["anybot-foundation"]),
    ]);
    let v = check_metadata_value(&m);
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(
        v.iter()
            .any(|s| s.contains("runner-test depends on anybot-runner: the fake computer helper"))
    );
    assert!(v.iter().any(|s| s.contains("not in the allow list")));
}

#[test]
fn test_depcheck_blocks_runner_test_as_a_normal_dependency() {
    let m = meta(vec![pkg("anybot-trust", &["runner-test"])]);
    let v = check_metadata_value(&m);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v.iter().any(|s| s.contains("only as a dev dependency")));
}
