use std::collections::BTreeSet;

use crate::tables::{allowed_internal_deps, disallowed_app_deps, forbidden_external_for_domain};

#[derive(serde::Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(serde::Deserialize)]
pub struct Package {
    pub name: String,
    pub source: Option<String>,
    pub dependencies: Vec<Dependency>,
}

#[derive(serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    pub kind: Option<String>,
}

fn normal_deps(pkg: &Package) -> BTreeSet<String> {
    pkg.dependencies
        .iter()
        .filter(|d| d.kind.as_deref() == Some("normal") || d.kind.is_none())
        .map(|d| d.name.clone())
        .collect()
}

fn check_app_package(pkg: &Package, deps: &BTreeSet<String>) -> Vec<String> {
    let mut violations = Vec::new();
    let Some(app_dir) = pkg.name.strip_prefix("anybot-app-") else {
        return violations;
    };
    for dep in deps {
        if dep == &pkg.name {
            violations.push(format!("{} 依賴自己（循環）", pkg.name));
            continue;
        }
        for dis in disallowed_app_deps(app_dir) {
            if *dep == format!("anybot-{dis}") {
                violations.push(format!(
                    "{} 依賴 {dep}：執行檔不准依賴（spec/07 §3）",
                    pkg.name
                ));
            }
        }
    }
    violations
}

fn check_internal_package(
    pkg: &Package,
    deps: &BTreeSet<String>,
    internal: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    let allowed = allowed_internal_deps();
    let Some(allow_set) = allowed.get(&pkg.name) else {
        violations.push(format!(
            "{}：depcheck 沒有這條依賴規則，請照 spec/07 §3 補進 allowed_internal_deps",
            pkg.name
        ));
        return violations;
    };
    for dep in deps {
        if dep == &pkg.name {
            violations.push(format!("{} 依賴自己（循環）", pkg.name));
            continue;
        }
        if internal.contains(dep) && !allow_set.contains(dep) {
            violations.push(format!(
                "{} 依賴 {dep}：准許清單裡沒有（spec/07 §3）",
                pkg.name
            ));
        }
        if pkg.name == "anybot-domain" && forbidden_external_for_domain().contains(dep) {
            violations.push(format!(
                "anybot-domain 依賴 {dep}：domain 不准碰網路、檔案、資料庫、非同步（spec/07 §3）",
            ));
        }
    }
    violations
}

fn check_harness_rules(
    pkg: &Package,
    deps: &BTreeSet<String>,
    internal: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    if pkg.name == "test-harness" {
        for dep in deps {
            if internal.contains(dep) {
                violations.push(format!(
                    "test-harness 依賴 {dep}：測試輔助套件不准依賴產品套件（spec/07 §3）"
                ));
            }
        }
        return violations;
    }
    if deps.contains("test-harness") {
        violations.push(format!(
            "{} 在正式依賴裡用 test-harness：harness 只准當開發用依賴（spec/09 K87）",
            pkg.name
        ));
    }
    violations
}

#[must_use]
pub fn check_metadata_value(meta: &Metadata) -> Vec<String> {
    let mut violations = Vec::new();
    let internal: BTreeSet<String> = meta
        .packages
        .iter()
        .filter(|p| p.source.is_none())
        .map(|p| p.name.clone())
        .collect();
    for pkg in &meta.packages {
        if pkg.source.is_some() {
            continue;
        }
        let deps = normal_deps(pkg);
        violations.extend(check_harness_rules(pkg, &deps, &internal));
        if pkg.name == "test-harness" {
            continue;
        }
        if pkg.name.starts_with("anybot-app-") {
            violations.extend(check_app_package(pkg, &deps));
        } else {
            violations.extend(check_internal_package(pkg, &deps, &internal));
        }
    }
    violations
}

pub fn check_metadata(json: &str) -> Result<Vec<String>, String> {
    let meta: Metadata =
        serde_json::from_str(json).map_err(|e| format!("無法解析 cargo metadata：{e}"))?;
    Ok(check_metadata_value(&meta))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                .any(|s| s.contains("anybot-foundation 依賴 anybot-protocol"))
        );
        assert!(
            v.iter()
                .any(|s| s.contains("anybot-server 依賴 anybot-runner"))
        );
        assert!(v.iter().any(|s| s.contains("anybot-domain 依賴 tokio")));
    }

    #[test]
    fn test_depcheck_blocks_harness_in_normal_deps_and_reverse() {
        let m = meta(vec![
            pkg("anybot-server", &["test-harness"]),
            pkg("anybot-foundation", &[]),
            pkg("test-harness", &["anybot-foundation"]),
            pkg("test-harness", &["serde"]),
        ]);
        let v = check_metadata_value(&m);
        assert_eq!(v.len(), 3);
        assert!(
            v.iter()
                .any(|s| s.contains("anybot-server 在正式依賴裡用 test-harness"))
        );
        assert!(
            v.iter()
                .any(|s| s.contains("test-harness 依賴 anybot-foundation"))
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
                .any(|s| s.contains("anybot-app-server 依賴 anybot-runner"))
        );
        assert!(
            v.iter()
                .any(|s| s.contains("anybot-app-runner-service 依賴 anybot-server"))
        );
        assert!(
            v.iter()
                .any(|s| s.contains("anybot-app-runner-service 依賴 anybot-store-server"))
        );
        assert!(
            v.iter()
                .any(|s| s.contains("anybot-app-cli 依賴 anybot-store-server"))
        );
        assert!(!v.iter().any(|s| s.contains("anybot-protocol")));
        assert!(!v.iter().any(|s| s.contains("anybot-sync")));
        assert!(
            !v.iter()
                .any(|s| s.contains("anybot-app-server 依賴 anybot-server"))
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
}
