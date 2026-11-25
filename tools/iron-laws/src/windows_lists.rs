use std::path::Path;

use crate::collect::read_file_lossy;

const TESTING_REL: &str = "docs/testing.md";
const HARNESS: &str = "test-harness";
const NO_TEST_MARKER: &str = "<!-- windows-no-test";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let path = repo.join(TESTING_REL);
    let Ok(text) = read_file_lossy(&path) else {
        return vec![format!("{TESTING_REL}: missing windows test lists")];
    };
    let listed = listed_names(&text, NO_TEST_MARKER);
    let mut actual = harness_dev_deps(repo);
    let mut out = Vec::new();
    for name in &listed {
        if !actual.iter().any(|found| found == name) {
            out.push(format!(
                "{TESTING_REL}: {name} is on windows-no-test but has no {HARNESS} dev-dependency"
            ));
        }
    }
    actual.retain(|name| !listed.iter().any(|item| item == name));
    for name in actual {
        out.push(format!(
            "{TESTING_REL}: {name} dev-depends on {HARNESS} but is not on windows-no-test"
        ));
    }
    out
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

fn harness_dev_deps(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for dir_name in ["crates", "apps", "tools"] {
        walk_manifests(&repo.join(dir_name), &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn walk_manifests(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_manifests(&path, out);
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
            continue;
        }
        let Ok(text) = read_file_lossy(&path) else {
            continue;
        };
        if dev_depends_on_harness(&text)
            && let Some(name) = package_name(&text)
        {
            out.push(name);
        }
    }
}

fn dev_depends_on_harness(text: &str) -> bool {
    let Some(body) = section_body(text, "[dev-dependencies]") else {
        return false;
    };
    body.lines().any(|line| {
        let key = line.split('=').next().unwrap_or("").trim();
        key == HARNESS
    })
}

fn package_name(text: &str) -> Option<String> {
    let body = section_body(text, "[package]")?;
    for line in body.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "name" {
            return Some(value.trim().trim_matches('"').to_owned());
        }
    }
    None
}

fn section_body(text: &str, header: &str) -> Option<String> {
    let mut body = String::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
            if trimmed == header {
                inside = true;
            } else if inside {
                break;
            }
            continue;
        }
        if inside {
            body.push_str(line);
            body.push('\n');
        }
    }
    if body.is_empty() { None } else { Some(body) }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch");
        let repo = root.join(name);
        std::fs::create_dir_all(repo.join("docs")).unwrap();
        std::fs::create_dir_all(repo.join("crates/demo")).unwrap();
        repo
    }

    fn write_pkg(repo: &Path, name: &str, dev: bool) {
        let dir = repo.join("crates").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let dev_line = if dev {
            "[dev-dependencies]\ntest-harness = { workspace = true }\n"
        } else {
            ""
        };
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n{dev_line}"),
        )
        .unwrap();
    }

    #[test]
    fn test_windows_lists_blocks_a_missing_and_an_extra_name() {
        let repo = scratch("iron-windows-lists");
        write_pkg(&repo, "needs-harness", true);
        write_pkg(&repo, "plain", false);
        std::fs::write(
            repo.join("docs/testing.md"),
            "<!-- windows-no-test\nplain\n-->\n",
        )
        .unwrap();
        let v = violations(&repo);
        assert!(
            v.iter()
                .any(|x| x.contains("needs-harness") && x.contains("not on")),
            "{v:?}"
        );
        assert!(
            v.iter()
                .any(|x| x.contains("plain") && x.contains("no test-harness")),
            "{v:?}"
        );
    }

    #[test]
    fn test_windows_lists_accepts_an_exact_match() {
        let repo = scratch("iron-windows-lists-ok");
        write_pkg(&repo, "needs-harness", true);
        std::fs::write(
            repo.join("docs/testing.md"),
            "<!-- windows-no-test\nneeds-harness\n-->\n",
        )
        .unwrap();
        assert_eq!(violations(&repo), Vec::<String>::new());
    }
}
