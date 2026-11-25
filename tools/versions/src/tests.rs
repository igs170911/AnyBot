use super::{VersionSources, violations};

fn sources<'a>(
    cargo: &'a str,
    tauri: &'a str,
    desktop: &'a str,
    root_package: &'a str,
    release: &'a str,
) -> VersionSources<'a> {
    VersionSources {
        cargo,
        tauri,
        desktop,
        root_package,
        release,
    }
}

fn aligned(version: &str) -> String {
    format!(
        "{{\n  \"productName\": \"AnyBot\",\n  \"version\": \"{version}\",\n  \"identifier\": \"app.anybot.desktop\"\n}}\n"
    )
}

const RELEASE: &str =
    "\"app.anybot.desktop\" \"app.anybot.desktop.dev\" \"AnyBot\" \"AnyBot Dev\"\n";

#[test]
fn versions_gate_accepts_matching_numbers() {
    let tauri = aligned("0.1.0");
    let found = violations(&sources(
        "[workspace.package]\nversion = \"0.1.0\"\n",
        &tauri,
        "{\"version\":\"0.1.0\"}\n",
        "{\"version\":\"0.1.0\"}\n",
        RELEASE,
    ));
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn versions_gate_blocks_a_tauri_mismatch() {
    let tauri = aligned("0.2.0");
    let found = violations(&sources(
        "[workspace.package]\nversion = \"0.1.0\"\n",
        &tauri,
        "{\"version\":\"0.1.0\"}\n",
        "{\"version\":\"0.1.0\"}\n",
        RELEASE,
    ));
    assert!(found.iter().any(|item| item.contains("0.2.0")), "{found:?}");
}

#[test]
fn versions_gate_blocks_a_changed_stable_identifier() {
    let tauri = aligned("0.1.0").replace("app.anybot.desktop", "app.anybot.other");
    let found = violations(&sources(
        "[workspace.package]\nversion = \"0.1.0\"\n",
        &tauri,
        "{\"version\":\"0.1.0\"}\n",
        "{\"version\":\"0.1.0\"}\n",
        RELEASE,
    ));
    assert!(
        found.iter().any(|item| item.contains("identifier")),
        "{found:?}"
    );
}
