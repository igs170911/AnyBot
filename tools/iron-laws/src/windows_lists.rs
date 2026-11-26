use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::collect::read_file_lossy;

const TESTING_REL: &str = "docs/testing.md";
const HARNESS: &str = "test-harness";
const NO_TEST_MARKER: &str = "<!-- windows-no-test";
const PRODUCT_ROOTS: [&str; 2] = ["crates", "apps"];
const TOOLS_ROOT: &str = "tools";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let path = repo.join(TESTING_REL);
    let Ok(text) = read_file_lossy(&path) else {
        return vec![format!("{TESTING_REL}: missing windows test lists")];
    };
    let listed = listed_names(&text, NO_TEST_MARKER);
    let Ok(metadata) = crate::compiled::cargo_metadata(repo) else {
        return vec![format!(
            "{TESTING_REL}: cargo metadata failed; cannot check windows-no-test"
        )];
    };
    let Ok(graph) = package_graph(&metadata) else {
        return vec![format!(
            "{TESTING_REL}: cargo metadata has no package graph"
        )];
    };
    let mut out = Vec::new();
    check_product(&graph, &listed, &mut out);
    check_tools(&graph, &listed, &mut out);
    out
}

fn check_product(graph: &BTreeMap<String, PackageNode>, listed: &[String], out: &mut Vec<String>) {
    let actual = harness_packages(graph);
    for name in listed {
        let Some(node) = graph.get(name) else {
            out.push(format!(
                "{TESTING_REL}: {name} is on windows-no-test but is not a workspace package"
            ));
            continue;
        };
        if node.root == "product" && !actual.iter().any(|found| found == name) {
            out.push(format!(
                "{TESTING_REL}: {name} is on windows-no-test but does not depend on {HARNESS}"
            ));
        }
    }
    for name in actual {
        if !listed.iter().any(|item| item == &name) {
            out.push(format!(
                "{TESTING_REL}: {name} depends on {HARNESS} but is not on windows-no-test"
            ));
        }
    }
}

fn harness_packages(graph: &BTreeMap<String, PackageNode>) -> Vec<String> {
    let mut actual: Vec<String> = graph
        .iter()
        .filter(|(_, node)| node.root == "product" && reaches_harness(graph, &node.deps))
        .map(|(name, _)| name.clone())
        .collect();
    actual.sort();
    actual
}

fn check_tools(graph: &BTreeMap<String, PackageNode>, listed: &[String], out: &mut Vec<String>) {
    for (name, node) in graph {
        if node.root != "tools" {
            continue;
        }
        if !listed.iter().any(|item| item == name) {
            out.push(format!(
                "{TESTING_REL}: {name} is a tools package but is not on windows-no-test"
            ));
        }
    }
}

struct PackageNode {
    root: &'static str,
    deps: Vec<String>,
}

fn package_graph(metadata_json: &str) -> Result<BTreeMap<String, PackageNode>, String> {
    let value: serde_json::Value =
        serde_json::from_str(metadata_json).map_err(|e| format!("cargo metadata json: {e}"))?;
    let packages = value
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("cargo metadata has no packages array")?;
    let mut graph = BTreeMap::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(manifest) = package
            .get("manifest_path")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let root = package_root(manifest);
        graph.insert(
            name.to_owned(),
            PackageNode {
                root,
                deps: dep_names(package),
            },
        );
    }
    Ok(graph)
}

fn package_root(manifest: &str) -> &'static str {
    let parts: Vec<&str> = manifest
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    let Some(index) = parts.iter().rposition(|part| *part == "Cargo.toml") else {
        return "other";
    };
    let Some(root) = index.checked_sub(2).and_then(|at| parts.get(at)) else {
        return "other";
    };
    root_kind(root)
}

fn root_kind(dir: &str) -> &'static str {
    if PRODUCT_ROOTS.contains(&dir) {
        "product"
    } else if dir == TOOLS_ROOT {
        "tools"
    } else {
        "other"
    }
}

fn dep_names(package: &serde_json::Value) -> Vec<String> {
    let Some(deps) = package
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut names = BTreeSet::new();
    for dep in deps {
        let Some(kind) = dep.get("kind").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if kind != "dev" && kind != "normal" && kind != "build" {
            continue;
        }
        let Some(name) = dep.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        names.insert(name.to_owned());
    }
    names.into_iter().collect()
}

fn reaches_harness(graph: &BTreeMap<String, PackageNode>, seeds: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<String> = seeds.to_vec();
    while let Some(name) = stack.pop() {
        if name == HARNESS {
            return true;
        }
        if !seen.insert(name.clone()) {
            continue;
        }
        let Some(node) = graph.get(&name) else {
            continue;
        };
        stack.extend(node.deps.iter().cloned());
    }
    false
}

fn listed_names(text: &str, marker: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line == marker {
            inside = true;
            continue;
        }
        if inside && line == "-->" {
            break;
        }
        if inside && !line.trim().is_empty() && !line.starts_with('#') {
            out.push(line.trim().to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_from(json: &str) -> BTreeMap<String, PackageNode> {
        package_graph(json).unwrap()
    }

    fn pkg(name: &str, manifest: &str, kind: &str, deps: &[&str]) -> String {
        let listed = deps
            .iter()
            .map(|dep| format!(r#"{{"name":"{dep}","kind":"{kind}"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"name":"{name}","manifest_path":"{manifest}","dependencies":[{listed}]}}"#)
    }

    #[test]
    fn test_windows_lists_graph_follows_indirect_dev_deps() {
        let json = format!(
            r#"{{"packages":[{},{},{}]}}"#,
            pkg("plain", "crates/plain/Cargo.toml", "dev", &[]),
            pkg(
                "needs-harness",
                "apps/needs/Cargo.toml",
                "dev",
                &["testenv"]
            ),
            pkg("testenv", "tests/testenv/Cargo.toml", "normal", &[HARNESS]),
        );
        let graph = graph_from(&json);
        let mut out = Vec::new();
        check_product(&graph, &["needs-harness".to_owned()], &mut out);
        assert_eq!(out, Vec::<String>::new());
        let mut missing = Vec::new();
        check_product(&graph, &[], &mut missing);
        assert!(
            missing.iter().any(|x| x.contains("needs-harness")),
            "{missing:?}"
        );
        assert!(!missing.iter().any(|x| x.contains("plain")), "{missing:?}");
    }

    #[test]
    fn test_windows_lists_graph_requires_every_tools_package() {
        let json = format!(
            r#"{{"packages":[{}]}}"#,
            pkg("iron-laws", "tools/iron-laws/Cargo.toml", "dev", &[]),
        );
        let graph = graph_from(&json);
        let mut missing = Vec::new();
        check_tools(&graph, &[], &mut missing);
        assert!(
            missing.iter().any(|x| x.contains("iron-laws")),
            "{missing:?}"
        );
        let mut listed = Vec::new();
        check_tools(&graph, &["iron-laws".to_owned()], &mut listed);
        assert_eq!(listed, Vec::<String>::new());
    }

    #[test]
    fn test_windows_lists_package_root_uses_either_separator() {
        assert_eq!(package_root(r"D:\repo\crates\server\Cargo.toml"), "product");
        assert_eq!(package_root("/repo/tools/iron-laws/Cargo.toml"), "tools");
        assert_eq!(package_root("/repo/tests/harness/Cargo.toml"), "other");
        assert_eq!(package_root("/repo/apps/server/Cargo.toml"), "product");
        assert_eq!(package_root("Cargo.toml"), "other");
    }
}
