use super::*;
use std::path::PathBuf;

fn scratch_dir(tag: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(tag);
    assert!(std::fs::create_dir_all(&repo).is_ok());
    repo
}

fn seed_license_files(repo: &Path) {
    assert!(std::fs::write(repo.join("LICENSE-MIT"), "text\n").is_ok());
    assert!(std::fs::write(repo.join("LICENSE-APACHE"), "text\n").is_ok());
}

fn clear_license_files(repo: &Path) {
    drop(std::fs::remove_file(repo.join("LICENSE-MIT")));
    drop(std::fs::remove_file(repo.join("LICENSE-APACHE")));
}

#[test]
fn test_licensing_missing_license_files_are_blocked() {
    let repo = scratch_dir("iron-licensing-files");
    clear_license_files(&repo);
    let v = violations(&repo);
    assert!(v.iter().any(|x| x.contains("LICENSE-MIT")));
    assert!(v.iter().any(|x| x.contains("LICENSE-APACHE")));
    seed_license_files(&repo);
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_licensing_cargo_toml_without_license_is_blocked() {
    let repo = scratch_dir("iron-licensing-cargo");
    seed_license_files(&repo);
    assert!(std::fs::create_dir_all(repo.join("crates/x")).is_ok());
    assert!(
        std::fs::write(
            repo.join("crates/x/Cargo.toml"),
            "[package]\nname = \"anybot-x\"\n"
        )
        .is_ok()
    );
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("crates/x/Cargo.toml") && x.contains("inherit"))
    );
    assert!(
        std::fs::write(
            repo.join("crates/x/Cargo.toml"),
            "[package]\nname = \"anybot-x\"\nlicense.workspace = true\n"
        )
        .is_ok()
    );
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_licensing_workspace_root_license_value_is_checked() {
    let repo = scratch_dir("iron-licensing-root");
    seed_license_files(&repo);
    assert!(std::fs::write(repo.join("Cargo.toml"), "[workspace]\n").is_ok());
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("Cargo.toml") && x.contains(DUAL_LICENSE))
    );
    assert!(
        std::fs::write(
            repo.join("Cargo.toml"),
            format!("[workspace.package]\nlicense = \"{DUAL_LICENSE}\"\n")
        )
        .is_ok()
    );
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_licensing_package_json_wrong_license_is_blocked() {
    let repo = scratch_dir("iron-licensing-json");
    seed_license_files(&repo);
    assert!(std::fs::create_dir_all(repo.join("web/ui")).is_ok());
    assert!(
        std::fs::write(
            repo.join("web/ui/package.json"),
            "{\"name\": \"ui\", \"license\": \"GPL-3.0\"}\n"
        )
        .is_ok()
    );
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("web/ui/package.json") && x.contains(DUAL_LICENSE))
    );
    assert!(
        std::fs::write(
            repo.join("web/ui/package.json"),
            format!("{{\"name\": \"ui\", \"license\": \"{DUAL_LICENSE}\"}}\n")
        )
        .is_ok()
    );
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_licensing_tauri_conf_bundle_license_is_checked() {
    let repo = scratch_dir("iron-licensing-tauri");
    seed_license_files(&repo);
    assert!(std::fs::create_dir_all(repo.join("apps/desktop/src-tauri")).is_ok());
    let conf = "{\"bundle\": {\"active\": false, \"targets\": \"all\"}}\n";
    assert!(std::fs::write(repo.join("apps/desktop/src-tauri/tauri.conf.json"), conf).is_ok());
    let v = violations(&repo);
    assert!(
        v.iter()
            .any(|x| x.contains("tauri.conf.json") && x.contains(DUAL_LICENSE))
    );
    assert!(
        std::fs::write(
            repo.join("apps/desktop/src-tauri/tauri.conf.json"),
            format!("{{\"bundle\": {{\"license\": \"{DUAL_LICENSE}\"}}}}\n")
        )
        .is_ok()
    );
    assert!(violations(&repo).is_empty());
}

#[test]
fn test_licensing_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(violations(&repo), Vec::<String>::new());
}
