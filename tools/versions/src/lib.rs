use serde_json as _;

pub const STABLE_IDENTIFIER: &str = "app.anybot.desktop";
pub const DEV_IDENTIFIER: &str = "app.anybot.desktop.dev";
pub const STABLE_APP_NAME: &str = "AnyBot";
pub const DEV_APP_NAME: &str = "AnyBot Dev";

pub struct VersionSources<'a> {
    pub cargo: &'a str,
    pub tauri: &'a str,
    pub desktop: &'a str,
    pub root_package: &'a str,
    pub release: &'a str,
}

#[must_use]
pub fn violations(sources: &VersionSources<'_>) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(version_agreement(sources));
    out.extend(channel_agreement(sources.release));
    out
}

fn version_agreement(sources: &VersionSources<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(workspace) = cargo_workspace_version(sources.cargo) else {
        out.push("Cargo.toml: missing [workspace.package] version".to_owned());
        return out;
    };
    compare(
        &mut out,
        "tauri.conf.json version",
        json_string(sources.tauri, "version").as_deref(),
        &workspace,
    );
    compare(
        &mut out,
        "apps/desktop/package.json version",
        json_string(sources.desktop, "version").as_deref(),
        &workspace,
    );
    compare(
        &mut out,
        "package.json version",
        json_string(sources.root_package, "version").as_deref(),
        &workspace,
    );
    if json_string(sources.tauri, "identifier").as_deref() != Some(STABLE_IDENTIFIER) {
        out.push(format!(
            "tauri.conf.json identifier must stay {STABLE_IDENTIFIER}; the dev channel overrides it at bundle time"
        ));
    }
    if json_string(sources.tauri, "productName").as_deref() != Some(STABLE_APP_NAME) {
        out.push("tauri.conf.json productName must stay AnyBot".to_owned());
    }
    out
}

fn channel_agreement(release: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (label, needle) in [
        ("stable identifier", STABLE_IDENTIFIER),
        ("dev identifier", DEV_IDENTIFIER),
        ("stable app name", STABLE_APP_NAME),
        ("dev app name", DEV_APP_NAME),
    ] {
        if !release.contains(&format!("\"{needle}\"")) {
            out.push(format!(
                "crates/foundation/src/release.rs: missing {label} \"{needle}\""
            ));
        }
    }
    out
}

fn compare(out: &mut Vec<String>, label: &str, found: Option<&str>, want: &str) {
    match found {
        Some(value) if value == want => {}
        Some(value) => out.push(format!("{label} is {value}, workspace version is {want}")),
        None => out.push(format!("{label} is missing")),
    }
}

fn cargo_workspace_version(text: &str) -> Option<String> {
    let mut in_package = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
            continue;
        }
        if in_package && let Some(value) = line.strip_prefix("version") {
            return toml_string(value);
        }
    }
    None
}

fn toml_string(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('=').trim();
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(value.to_owned())
}

fn json_string(text: &str, field: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value.get(field)?.as_str().map(str::to_owned)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
