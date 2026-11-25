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
            violations.push(format!("{} depends on itself (cycle)", pkg.name));
            continue;
        }
        for dis in disallowed_app_deps(app_dir) {
            if *dep == format!("anybot-{dis}") {
                violations.push(format!(
                    "{} depends on {dep}: apps must not depend (spec/07 section 3)",
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
            "{}: depcheck has no rule for this dependency; add it to allowed_internal_deps per spec/07 section 3",
            pkg.name
        ));
        return violations;
    };
    for dep in deps {
        if dep == &pkg.name {
            violations.push(format!("{} depends on itself (cycle)", pkg.name));
            continue;
        }
        if internal.contains(dep) && !allow_set.contains(dep) {
            violations.push(format!(
                "{} depends on {dep}: not in the allow list (spec/07 section 3)",
                pkg.name
            ));
        }
        if pkg.name == "anybot-domain" && forbidden_external_for_domain().contains(dep) {
            violations.push(format!(
                "anybot-domain depends on {dep}: domain must not touch network, files, databases, async (spec/07 section 3)",
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
            if dep == "anybot-foundation" {
                continue;
            }
            if internal.contains(dep) {
                violations.push(format!(
                    "test-harness depends on {dep}: the test harness must not depend on product crates (spec/07 section 3)"
                ));
            }
        }
        return violations;
    }
    if deps.contains("test-harness") && !HARNESS_RUNTIME_USERS.contains(&pkg.name.as_str()) {
        violations.push(format!(
            "{} uses test-harness in normal dependencies: the harness is allowed only as a dev dependency (spec/09 K87)",
            pkg.name
        ));
    }
    violations
}

const HARNESS_RUNTIME_USERS: [&str; 2] = ["testenv", "live-model"];

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
        serde_json::from_str(json).map_err(|e| format!("cannot parse cargo metadata: {e}"))?;
    Ok(check_metadata_value(&meta))
}

#[cfg(test)]
mod metadata_tests;
