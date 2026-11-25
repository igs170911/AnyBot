use super::*;
use std::path::PathBuf;

fn ws_fixture() -> Vec<(String, String, String)> {
    vec![
        (
            "rust".to_owned(),
            "unsafe_code".to_owned(),
            "\"forbid\"".to_owned(),
        ),
        (
            "clippy".to_owned(),
            "exit".to_owned(),
            "\"deny\"".to_owned(),
        ),
        (
            "clippy".to_owned(),
            "panic".to_owned(),
            "\"deny\"".to_owned(),
        ),
        (
            "clippy".to_owned(),
            "all".to_owned(),
            "{ level = \"warn\", priority = -2 }".to_owned(),
        ),
    ]
}

fn context_manifest(with_extra_disable: bool) -> String {
    let mut t = "[package]\nname = \"anybot-app-desktop-context\"\n\n".to_owned();
    t += "[lints.rust]\nunsafe_code = \"forbid\"\n\n[lints.clippy]\n";
    t += "panic = \"deny\"\n";
    if !with_extra_disable {
        t += "all = { level = \"warn\", priority = -2 }\n";
    }
    t
}

#[test]
fn test_lints_lint_entries_parses_sections_and_skips_others() {
    let text = "[workspace.lints.rust]\nunsafe_code = \"forbid\"\n[workspace.lints.clippy]\nall = { level = \"warn\", priority = -2 }\n[dependencies]\ntokio = \"1\"\n";
    let e = lint_entries(text, "[workspace.lints.rust]", "[workspace.lints.clippy]");
    assert_eq!(
        e,
        vec![
            (
                "rust".to_owned(),
                "unsafe_code".to_owned(),
                "\"forbid\"".to_owned()
            ),
            (
                "clippy".to_owned(),
                "all".to_owned(),
                "{ level = \"warn\", priority = -2 }".to_owned(),
            ),
        ]
    );
}

#[test]
fn test_lints_inherits_workspace_detects_only_in_lints_section() {
    assert!(inherits_workspace("[lints]\nworkspace = true\n"));
    assert!(!inherits_workspace("[dependencies]\nworkspace = true\n"));
    assert!(!inherits_workspace("[lints]\nworkspace = false\n"));
}

#[test]
fn test_lints_member_without_inherit_is_blocked() {
    let manifests = [(
        "crates/x/Cargo.toml".to_owned(),
        "[package]\nname = \"anybot-x\"\n".to_owned(),
    )];
    let v = check_members(&manifests, &ws_fixture(), &[]);
    assert!(
        v.first()
            .is_some_and(|x| x.contains("沒有 [lints] workspace = true"))
    );
}

#[test]
fn test_lints_context_extra_disabled_lint_is_blocked() {
    let manifests = [(
        "apps/desktop/context/Cargo.toml".to_owned(),
        context_manifest(true),
    )];
    let files = [(
        "apps/desktop/context/src/lib.rs".to_owned(),
        "pub fn make_context() {}\n".to_owned(),
    )];
    let v = check_members(&manifests, &ws_fixture(), &files);
    assert!(v.iter().any(|x| x.contains("少了 clippy.all")));
    assert!(!v.iter().any(|x| x.contains("少了 clippy.exit")));
}

#[test]
fn test_lints_context_missing_exit_entry_is_ok() {
    let manifests = [(
        "apps/desktop/context/Cargo.toml".to_owned(),
        context_manifest(false),
    )];
    let files = [(
        "apps/desktop/context/src/lib.rs".to_owned(),
        "pub fn make_context() {}\n".to_owned(),
    )];
    let v = check_members(&manifests, &ws_fixture(), &files);
    assert!(v.is_empty());
}

#[test]
fn test_lints_context_extra_code_is_blocked() {
    let manifests = [(
        "apps/desktop/context/Cargo.toml".to_owned(),
        context_manifest(false),
    )];
    let files = [
        (
            "apps/desktop/context/src/lib.rs".to_owned(),
            "pub fn make_context() {}\nfn helper() {}\n".to_owned(),
        ),
        (
            "apps/desktop/context/src/extra.rs".to_owned(),
            "pub fn other() {}\n".to_owned(),
        ),
    ];
    let v = check_members(&manifests, &ws_fixture(), &files);
    assert!(v.first().is_some_and(|x| x.contains("只准有那一個函式")));
}

#[test]
fn test_lints_context_using_workspace_true_is_blocked() {
    let manifests = [(
        "apps/desktop/context/Cargo.toml".to_owned(),
        "[package]\nname = \"anybot-app-desktop-context\"\n[lints]\nworkspace = true\n".to_owned(),
    )];
    let files = [(
        "apps/desktop/context/src/lib.rs".to_owned(),
        "pub fn make_context() {}\n".to_owned(),
    )];
    let v = check_members(&manifests, &ws_fixture(), &files);
    assert!(
        v.iter()
            .any(|x| x.contains("不准用 [lints] workspace = true"))
    );
}

#[test]
fn test_lints_gitleaks_allowlist_is_blocked() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch/iron-lints");
    assert!(std::fs::create_dir_all(&repo).is_ok());
    assert!(std::fs::write(repo.join(".gitleaks.toml"), "[allowlist]\n").is_ok());
    let v = gitleaks_violations(&repo);
    assert!(v.first().is_some_and(|x| x.contains("不准有任何放行清單")));
    assert!(std::fs::remove_file(repo.join(".gitleaks.toml")).is_ok());
    assert!(gitleaks_violations(&repo).is_empty());
}

#[test]
fn test_lints_member_globs_expand_crates_and_apps() {
    let root = "members = [\n    \"crates/*\",\n    \"apps/server\",\n    \"tools/*\",\n]\n";
    let globs = member_globs(root);
    assert_eq!(globs, ["crates/*", "apps/server", "tools/*"]);
}

#[test]
fn test_lints_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new());
}

#[test]
fn test_lints_real_repo_context_has_one_function() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = crate::collect::source_files(&repo);
    assert_eq!(ctx_function_count(&files), 1);
}

#[test]
fn test_lints_edition_check_accepts_workspace_inheritance_and_flags_literals() {
    let root = "[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n";
    let good = "[package]\nedition.workspace = true\nversion.workspace = true\n";
    let bad = "[package]\nedition = \"2021\"\n";
    let missing = "[package]\nname = \"x\"\n";
    let manifests = vec![
        ("crates/a/Cargo.toml".to_owned(), good.to_owned()),
        ("crates/b/Cargo.toml".to_owned(), bad.to_owned()),
        ("crates/c/Cargo.toml".to_owned(), missing.to_owned()),
    ];
    let v = crate::editions::edition_violations(root, &manifests);
    assert_eq!(v.len(), 2);
    assert!(
        v.iter()
            .any(|s| s.contains("crates/b") && s.contains("2021") && s.contains("2024"))
    );
    assert!(
        v.iter()
            .any(|s| s.contains("crates/c") && s.contains("沒有寫"))
    );
}

#[test]
fn test_lints_edition_check_needs_workspace_base() {
    let root = "[workspace.package]\nversion = \"0.1.0\"\n";
    let v = crate::editions::edition_violations(root, &[]);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|s| s.contains("沒有寫")));
}
