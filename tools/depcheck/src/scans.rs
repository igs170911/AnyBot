use crate::tables::packages_allowed_exemptions;

#[must_use]
pub fn env_usage_violations(files: &[(String, String)]) -> Vec<String> {
    const PATS: [&str; 4] = ["env::var", "env::var_os", "env::vars", "env::vars_os"];
    let mut violations = Vec::new();
    for (path, content) in files {
        if !(path.starts_with("crates/") || path.starts_with("apps/")) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            if PATS.iter().any(|pat| line.contains(pat)) {
                violations.push(format!(
                    "{path}:{}: reading environment variables is banned everywhere (spec/07 section 3)",
                    i.saturating_add(1)
                ));
            }
        }
    }
    violations
}

#[must_use]
pub fn package_of_path(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let head = parts.next()?;
    let dir = parts.next()?;
    match (head, parts.next()) {
        ("crates", Some(_)) => Some(format!("anybot-{dir}")),
        ("apps", Some(_)) => Some(format!("anybot-app-{dir}")),
        ("tools", Some(_)) => Some(dir.to_owned()),
        _ => None,
    }
}

#[must_use]
pub fn scan_exemption_attrs(files: &[(String, String)]) -> Vec<String> {
    let mut violations = Vec::new();
    let allowed = packages_allowed_exemptions();
    for (path, content) in files {
        let Some(pkg) = package_of_path(path) else {
            continue;
        };
        if allowed.contains(&pkg) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            let t = line.replace(' ', "");
            if t.contains("allow(clippy::disallowed_methods")
                || t.contains("allow(clippy::disallowed_types")
            {
                violations.push(format!(
                    "{path}:{}: allow(clippy::disallowed_...) may appear only in exempt packages (now in {pkg}, spec/07 section 3)",
                    i.saturating_add(1)
                ));
            }
        }
    }
    violations
}

fn package_name_in_toml(content: &str) -> Option<String> {
    let in_package = content
        .lines()
        .skip_while(|l| !l.trim().eq("[package]"))
        .skip(1)
        .take_while(|l| !l.trim().starts_with('['));
    for line in in_package {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name = \"") {
            return Some(rest.trim_end_matches('"').to_owned());
        }
    }
    None
}

fn feature_list_of(content: &str, dep: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let t = line.replace(' ', "");
        let prefix = format!("{dep}=");
        let Some((_, after)) = t.split_once(&prefix) else {
            continue;
        };
        let Some((_, feats)) = after.split_once("features=[") else {
            continue;
        };
        let list = feats.split_once(']').map_or(feats, |(l, _)| l);
        for f in list.split(',') {
            let clean = f.trim_matches('"').to_owned();
            if !clean.is_empty() {
                out.push(clean);
            }
        }
    }
    out
}

fn http_client_names(content: &str) -> Vec<String> {
    const CLIENTS: [&str; 5] = ["reqwest", "ureq", "isahc", "attohttpc", "curl"];
    let mut out = Vec::new();
    for line in content.lines() {
        let t = line.replace(' ', "");
        for c in CLIENTS {
            let direct = format!("{c}=");
            let quoted = format!("\"{c}\"");
            if t.starts_with(&direct) || t.contains(&quoted) {
                out.push(c.to_owned());
            }
        }
    }
    out
}

const TOKIO_PROCESS_ALLOWED: [&str; 2] = ["anybot-computers", "test-harness"];
const TOKIO_FS_ALLOWED: [&str; 5] = [
    "anybot-computers",
    "anybot-blob",
    "anybot-store-client",
    "anybot-foundation",
    "test-harness",
];
const HTTP_CLIENT_ALLOWED: [&str; 7] = [
    "anybot-models",
    "anybot-mcp",
    "anybot-sync",
    "anybot-channels",
    "anybot-computers",
    "test-harness",
    "live-model",
];

#[must_use]
pub fn cargo_toml_violations(files: &[(String, String)]) -> Vec<String> {
    let mut violations = Vec::new();
    for (path, content) in files {
        if !path.ends_with("Cargo.toml") {
            continue;
        }
        let Some(pkg) = package_name_in_toml(content) else {
            continue;
        };
        for f in feature_list_of(content, "tokio") {
            if f == "process" && !TOKIO_PROCESS_ALLOWED.contains(&pkg.as_str()) {
                violations.push(format!(
                    "{path}: tokio process features are allowed only in {TOKIO_PROCESS_ALLOWED:?} (spec/07 section 3)"
                ));
            }
            if f == "fs" && !TOKIO_FS_ALLOWED.contains(&pkg.as_str()) {
                violations.push(format!(
                    "{path}: tokio fs features are allowed only in {TOKIO_FS_ALLOWED:?} (spec/07 section 3)"
                ));
            }
        }
        for c in http_client_names(content) {
            if !HTTP_CLIENT_ALLOWED.contains(&pkg.as_str()) {
                violations.push(format!(
                    "{path}: HTTP client crate {c} is allowed only in {HTTP_CLIENT_ALLOWED:?} (spec/07 section 3)"
                ));
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow_attr_fixture() -> String {
        format!("#!{}(clippy::disallowed_methods)]\n", "allow")
    }

    #[test]
    fn test_depcheck_blocks_bad_attr() {
        let files = vec![
            ("crates/domain/src/lib.rs".to_owned(), allow_attr_fixture()),
            (
                "crates/computers/src/lib.rs".to_owned(),
                allow_attr_fixture(),
            ),
            (
                "apps/cli/src/main.rs".to_owned(),
                format!("{}(clippy::disallowed_types) in apps is banned\n", "allow"),
            ),
        ];
        let v = scan_exemption_attrs(&files);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|s| s.contains("crates/domain/src/lib.rs:1")));
        assert!(v.iter().any(|s| s.contains("apps/cli/src/main.rs:1")));
    }

    #[test]
    fn test_depcheck_blocks_env_usage() {
        let files = vec![
            (
                "crates/foundation/src/lib.rs".to_owned(),
                "let x = std::env::var(\"A\");\n".to_owned(),
            ),
            (
                "apps/cli/src/main.rs".to_owned(),
                "std::env::vars_os();\n".to_owned(),
            ),
            (
                "tools/spec-trace/src/main.rs".to_owned(),
                "std::env::args();\n".to_owned(),
            ),
            (
                "crates/bus/src/lib.rs".to_owned(),
                format!(
                    "{}reading parameters is allowed; nothing is disallowed here.\n",
                    "//"
                ),
            ),
        ];
        let v = env_usage_violations(&files);
        assert_eq!(v.len(), 2);
        assert!(
            v.iter()
                .any(|s| s.contains("crates/foundation/src/lib.rs:1"))
        );
        assert!(v.iter().any(|s| s.contains("apps/cli/src/main.rs:1")));
    }

    #[test]
    fn test_depcheck_blocks_extra_features_and_http_clients() {
        let files = vec![
            (
                "crates/server/Cargo.toml".to_owned(),
                "[package]\nname = \"anybot-server\"\ntokio = { workspace = true, features = [\"process\", \"fs\", \"net\"] }\nreqwest = { workspace = true }\n".to_owned(),
            ),
            (
                "crates/computers/Cargo.toml".to_owned(),
                "[package]\nname = \"anybot-computers\"\ntokio = { workspace = true, features = [\"process\"] }\nreqwest = { workspace = true }\n".to_owned(),
            ),
            (
                "tests/harness/Cargo.toml".to_owned(),
                "[package]\nname = \"test-harness\"\ntokio = { workspace = true, features = [\"fs\", \"process\"] }\nureq = \"2\"\n".to_owned(),
            ),
            (
                "Cargo.toml".to_owned(),
                "[workspace]\n[workspace.dependencies]\ntokio = { version = \"1\", features = [\"macros\", \"rt\", \"net\"] }\nreqwest = { version = \"0.12\", features = [\"rustls\"] }\n".to_owned(),
            ),
        ];
        let v = cargo_toml_violations(&files);
        assert_eq!(v.len(), 3);
        assert!(v.iter().any(|s| s.contains("crates/server/Cargo.toml")));
        assert!(v.iter().all(|s| s.contains("allowed only")));
    }

    #[test]
    fn test_depcheck_package_of_path_covers_three_heads() {
        assert_eq!(
            package_of_path("crates/domain/src/lib.rs").as_deref(),
            Some("anybot-domain")
        );
        assert_eq!(
            package_of_path("apps/cli/src/main.rs").as_deref(),
            Some("anybot-app-cli")
        );
        assert_eq!(
            package_of_path("tools/depcheck/src/main.rs").as_deref(),
            Some("depcheck")
        );
        assert_eq!(package_of_path("web/ui/src/index.ts"), None);
    }
}
