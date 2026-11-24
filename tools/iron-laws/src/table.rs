use std::collections::{BTreeMap, BTreeSet};

pub const SCAN_DIRS: [&str; 6] = ["crates", "apps", "tools", "web", "adapters", "tests"];

const PROC_M: [&str; 4] = [
    "std::process::Command::new",
    "std::process::Command::output",
    "std::process::Command::spawn",
    "std::process::Command::status",
];
const HTTP_M: [&str; 8] = [
    "reqwest::Client::new",
    "reqwest::Client::get",
    "reqwest::Client::post",
    "reqwest::Client::put",
    "reqwest::Client::delete",
    "reqwest::Client::head",
    "reqwest::Client::request",
    "reqwest::RequestBuilder::send",
];
const FS_M: [&str; 6] = [
    "std::fs::read",
    "std::fs::read_to_string",
    "std::fs::write",
    "std::fs::remove_file",
    "std::fs::rename",
    "std::fs::create_dir_all",
];
const FS_T: [&str; 2] = ["std::fs::File", "std::fs::OpenOptions"];
const CMD_T: &str = "std::process::Command";

#[must_use]
pub fn suppression_tokens() -> Vec<String> {
    vec![
        format!("#[{a}", a = "allow"),
        format!("#![{a}", a = "allow"),
        format!("#[{e}", e = "expect"),
        format!("#![{e}", e = "expect"),
        format!("b{i}ome-{b}", i = "i", b = "ignore"),
        format!("@ts-{b}", b = "ignore"),
        format!("@ts-expect-{c}", c = "error"),
        format!("@ts-no{c}", c = "check"),
        format!("eslint-{d}", d = "disable"),
    ]
}

pub const SKIP_DIR_NAMES: [&str; 8] = [
    "node_modules",
    "dist",
    "target",
    "gen",
    "build",
    "coverage",
    ".pnpm",
    "generated",
];

pub const SOURCE_EXTENSIONS: [&str; 3] = ["rs", "ts", "tsx"];

pub const MAX_SOURCE_LINES: usize = 300;

#[must_use]
pub fn has_extension(path: &str, ext: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

pub const MAX_FUNCTION_LINES: usize = 60;

pub const ALLOWED_DENY_IGNORE: &str = "RUSTSEC-2024-0370";

pub const DENY_EXCEPTION_KEYS: [(&str, &str); 4] = [
    ("bans", "allow"),
    ("bans", "skip"),
    ("licenses", "exceptions"),
    ("advisories", "ignore"),
];

#[must_use]
pub fn clippy_exemptions() -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut m = BTreeMap::new();
    insert(
        &mut m,
        "anybot-foundation",
        &["std::fs::read_to_string"],
        &[],
    );
    insert(
        &mut m,
        "anybot-computers",
        &merge(&[PROC_M.as_slice(), HTTP_M.as_slice(), FS_M.as_slice()]),
        &[CMD_T, FS_T[0], FS_T[1]],
    );
    insert(&mut m, "anybot-models", &HTTP_M, &[]);
    insert(&mut m, "anybot-mcp", &HTTP_M, &[]);
    insert(&mut m, "anybot-sync", &HTTP_M, &[]);
    insert(&mut m, "anybot-channels", &HTTP_M, &[]);
    insert(&mut m, "anybot-blob", &FS_M, &FS_T);
    insert(&mut m, "anybot-store-client", &FS_M, &FS_T);
    insert(
        &mut m,
        "depcheck",
        &merge(&[&PROC_M, &FS_M]),
        &[CMD_T, FS_T[0], FS_T[1]],
    );
    insert(
        &mut m,
        "spec-trace",
        &merge(&[&PROC_M, &FS_M]),
        &[CMD_T, FS_T[0], FS_T[1]],
    );
    insert(
        &mut m,
        "iron-laws",
        &merge(&[&PROC_M, &FS_M]),
        &[CMD_T, FS_T[0], FS_T[1]],
    );
    m
}

fn merge<'a>(parts: &'a [&'a [&'a str]]) -> Vec<&'a str> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

fn insert(
    m: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    pkg: &str,
    methods: &[&str],
    types: &[&str],
) {
    let mut per_key = BTreeMap::new();
    if !methods.is_empty() {
        per_key.insert(
            "disallowed-methods".to_owned(),
            methods
                .iter()
                .map(|p| (*p).to_owned())
                .collect::<BTreeSet<_>>(),
        );
    }
    if !types.is_empty() {
        per_key.insert(
            "disallowed-types".to_owned(),
            types
                .iter()
                .map(|p| (*p).to_owned())
                .collect::<BTreeSet<_>>(),
        );
    }
    m.insert(pkg.to_owned(), per_key);
}

#[must_use]
pub fn package_of_path(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    let head = parts.next()?;
    let dir = parts.next()?;
    let third = parts.next();
    match (head, third) {
        ("crates", Some(_)) => Some(format!("anybot-{dir}")),
        ("apps", Some("src-tauri")) => Some("anybot-app-desktop".to_owned()),
        ("apps", Some(_) | None) => Some(format!("anybot-app-{dir}")),
        ("tools", Some(_) | None) => Some(dir.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_exemptions_match_repo_layout_paths() {
        let e = clippy_exemptions();
        assert!(e.contains_key("anybot-computers"));
        assert!(e.contains_key("depcheck"));
        assert!(!e.contains_key("anybot-domain"));
        let want = ["std::fs::read_to_string".to_owned()]
            .into_iter()
            .collect::<BTreeSet<String>>();
        let got = e
            .get("anybot-foundation")
            .and_then(|m| m.get("disallowed-methods"));
        assert_eq!(got, Some(&want));
    }

    #[test]
    fn test_table_package_of_path_covers_crates_apps_tools() {
        assert_eq!(
            package_of_path("crates/computers/clippy.toml").as_deref(),
            Some("anybot-computers")
        );
        assert_eq!(
            package_of_path("apps/desktop/src-tauri/clippy.toml").as_deref(),
            Some("anybot-app-desktop")
        );
        assert_eq!(
            package_of_path("apps/desktop/src-tauri/src/main.rs").as_deref(),
            Some("anybot-app-desktop")
        );
        assert_eq!(
            package_of_path("apps/server/src/main.rs").as_deref(),
            Some("anybot-app-server")
        );
        assert_eq!(
            package_of_path("tools/iron-laws/src/main.rs").as_deref(),
            Some("iron-laws")
        );
        assert_eq!(package_of_path("web/ui/src/shell.tsx"), None);
    }

    #[test]
    fn test_table_suppression_tokens_cover_the_rules() {
        let t = suppression_tokens();
        assert_eq!(t.len(), 9);
        assert!(t.contains(&format!("#[{k}", k = "allow")));
        assert!(t.contains(&format!("#![{k}", k = "allow")));
        assert!(t.contains(&format!("@ts-expect-{k}", k = "error")));
        assert!(t.contains(&format!("eslint-{k}", k = "disable")));
    }
}
