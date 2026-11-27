use std::path::Path;

use crate::collect::{is_skipped_dir, read_file_lossy};
use crate::nextest_config::{NEXTEST_CONFIG_REL, quick_filter_of};

pub const NAME_MARK: &str = "compile_fail";
const QUICK_MARK: &str = "test(/_compile_fail$/)";
const SCAN_ROOTS: [&str; 4] = ["crates", "apps", "tools", "tests"];
const TOOLS: [&str; 4] = ["trybuild", "cargo", "rustc", "clippy-driver"];
const COMPILE_ARGS: [&str; 5] = ["clippy", "check", "build", "test", "rustc"];

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = quick_filter_violations(repo);
    for root in SCAN_ROOTS {
        scan_dir(&repo.join(root), repo, &mut out);
    }
    out.sort();
    out
}

fn quick_filter_violations(repo: &Path) -> Vec<String> {
    let path = repo.join(NEXTEST_CONFIG_REL);
    let Ok(content) = read_file_lossy(&path) else {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: missing; basic tests must skip {NAME_MARK} tests (docs/testing.md)"
        )];
    };
    let Some(filter) = quick_filter_of(&content) else {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: [profile.quick] has no default-filter; basic tests must skip {NAME_MARK} tests (docs/testing.md)"
        )];
    };
    if filter.contains(QUICK_MARK) {
        return Vec::new();
    }
    vec![format!(
        "{NEXTEST_CONFIG_REL}: quick profile must exclude {QUICK_MARK} (docs/testing.md)"
    )]
}

fn scan_dir(dir: &Path, repo: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        scan_entry(&entry.path(), repo, out);
    }
}

fn scan_entry(path: &Path, repo: &Path, out: &mut Vec<String>) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    if path.is_dir() {
        if !is_skipped_dir(name) {
            scan_dir(path, repo, out);
        }
        return;
    }
    scan_rs(path, repo, name, out);
}

fn scan_rs(path: &Path, repo: &Path, name: &str, out: &mut Vec<String>) {
    if Path::new(name)
        .extension()
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("rs"))
    {
        return;
    }
    let Ok(text) = read_file_lossy(path) else {
        return;
    };
    out.extend(file_violations(&rel_of(path, repo, name), &text));
}

fn rel_of(path: &Path, repo: &Path, name: &str) -> String {
    path.strip_prefix(repo)
        .map_or_else(|_| name.to_owned(), |p| p.display().to_string())
}

#[must_use]
pub fn file_violations(path: &str, content: &str) -> Vec<String> {
    if path.split('/').any(|part| part == "compilefail") {
        return Vec::new();
    }
    let stripped = crate::comments::strip_rust_strings(content);
    let mut out = Vec::new();
    for (index, line) in stripped.lines().enumerate() {
        let trimmed = line.trim();
        if !fn_head(trimmed) || !test_fn(&stripped, index) {
            continue;
        }
        let Some(name) = fn_name(trimmed) else {
            continue;
        };
        if name.ends_with(NAME_MARK) {
            continue;
        }
        let body = fn_body(content, index);
        if body_spawns_compiler(body) {
            out.push(format!(
                "{path}:{} test `{name}` spawns a compiler but its name lacks `{NAME_MARK}` (docs/testing.md)",
                index.saturating_add(1)
            ));
        }
    }
    out
}

fn test_fn(content: &str, index: usize) -> bool {
    let prior: Vec<&str> = content
        .lines()
        .take(index)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut saw = false;
    for line in prior.into_iter().rev() {
        if !line.starts_with('#') {
            break;
        }
        if line.contains("#[test]") || line.contains("::test") {
            saw = true;
        }
    }
    saw
}

fn fn_head(head: &str) -> bool {
    head.starts_with("fn ")
        || head.starts_with("pub fn ")
        || head.starts_with("async fn ")
        || head.starts_with("pub async fn ")
        || head.starts_with("pub(crate) fn ")
        || head.starts_with("pub(crate) async fn ")
}

fn fn_name(head: &str) -> Option<&str> {
    let after = head.split_once("fn ")?.1;
    let name = after.split(['(', '<', ' ']).next()?.trim();
    if name.is_empty() { None } else { Some(name) }
}

fn fn_end(lines: &[&str], from: usize) -> usize {
    let mut depth = 0i32;
    for (offset, line) in lines.iter().skip(from).enumerate() {
        depth = depth.saturating_add(depth_delta(line));
        let idx = from.saturating_add(offset);
        if depth == 0 && idx > from && line.contains('}') {
            return idx.saturating_add(1);
        }
    }
    lines.len()
}

fn fn_body(content: &str, from: usize) -> &str {
    let lines: Vec<&str> = content.lines().collect();
    let end = fn_end(&lines, from);
    let start_byte = byte_at(&lines, from);
    let end_byte = byte_at(&lines, end);
    content.get(start_byte..end_byte).unwrap_or("")
}

fn byte_at(lines: &[&str], count: usize) -> usize {
    lines
        .iter()
        .take(count)
        .map(|line| line.len().saturating_add(1))
        .sum()
}

fn depth_delta(line: &str) -> i32 {
    let open = i32::try_from(line.matches('{').count()).unwrap_or(i32::MAX);
    let close = i32::try_from(line.matches('}').count()).unwrap_or(i32::MAX);
    open.saturating_sub(close)
}

fn body_spawns_compiler(body: &str) -> bool {
    let code = crate::comments::strip_rust_strings(body);
    if code.contains("trybuild::") {
        return true;
    }
    TOOLS
        .iter()
        .filter(|tool| **tool != "trybuild")
        .any(|tool| call_site(body, tool))
}

fn call_site(body: &str, tool: &str) -> bool {
    quoted_tool(body, tool) && (tool != "cargo" || cargo_compiles(body))
}

fn quoted_tool(body: &str, tool: &str) -> bool {
    body.contains(&format!("\"{tool}\"")) || body.contains(&format!("\"{tool}.exe\""))
}

fn cargo_compiles(body: &str) -> bool {
    COMPILE_ARGS
        .iter()
        .any(|arg| body.contains(&format!("\"{arg}\"")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_repo(name: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch");
        let repo = root.join(name);
        drop(std::fs::remove_dir_all(&repo));
        std::fs::create_dir_all(repo.join(".config")).unwrap();
        std::fs::write(
            repo.join(".config").join("nextest.toml"),
            "[profile.quick]\ndefault-filter = 'not (binary(/^pg_/) | test(/_compile_fail$/))'\n",
        )
        .unwrap();
        repo
    }

    #[test]
    fn test_compile_fail_names_blocks_a_misnamed_spawn() {
        let repo = scratch_repo("iron-compile-fail-name");
        let dir = repo.join("crates").join("widget").join("src");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lib.rs"),
            "#[test]\nfn plant_a_bad_compile() {\n    std::process::Command::new(\"cargo\").arg(\"clippy\").output().ok();\n}\n",
        )
        .unwrap();
        let found = violations(&repo);
        assert!(
            found
                .iter()
                .any(|row| row.contains("plant_a_bad_compile") && row.contains(NAME_MARK)),
            "{found:?}"
        );
    }

    #[test]
    fn test_compile_fail_names_accepts_the_mark_and_fixture_dirs() {
        let repo = scratch_repo("iron-compile-fail-ok");
        let src = repo.join("crates").join("widget").join("src");
        let fixture = repo
            .join("crates")
            .join("widget")
            .join("tests")
            .join("compilefail");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&fixture).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            "#[test]\nfn secret_serialization_compile_fail() {\n    trybuild::TestCases::new().compile_fail(\"tests/compilefail/*.rs\");\n}\n",
        )
        .unwrap();
        std::fs::write(
            fixture.join("bad.rs"),
            "fn main() {\n    std::process::Command::new(\"rustc\").status().ok();\n}\n",
        )
        .unwrap();
        let found = violations(&repo);
        assert_eq!(found, Vec::<String>::new(), "{found:?}");
    }

    #[test]
    fn test_compile_fail_names_blocks_a_mark_that_is_not_the_suffix() {
        let planted = "#[test]\nfn compile_fail_is_not_the_suffix() {\n    trybuild::TestCases::new().compile_fail(\"x.rs\");\n}\n";
        let found = file_violations("crates/widget/src/lib.rs", planted);
        assert!(
            found
                .iter()
                .any(|row| row.contains("compile_fail_is_not_the_suffix")),
            "{found:?}"
        );
    }

    #[test]
    fn test_compile_fail_names_real_repo_is_clean() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let found = violations(&repo);
        assert_eq!(found, Vec::<String>::new(), "{found:?}");
    }
}
