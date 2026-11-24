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
                    "{path}:{}：讀環境變數任何地方都不准（spec/07 §3）",
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
                    "{path}:{}：allow(clippy::disallowed_…) 只准出現在准許的套件（現在在 {pkg}，spec/07 §3）",
                    i.saturating_add(1)
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
                format!("{}(clippy::disallowed_types) 在 apps 不准\n", "allow"),
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
                format!("{}令參數可以讀；這裡沒有禁用。\n", "//"),
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
