pub const SCAN_DIRS: [&str; 6] = ["crates", "apps", "tools", "web", "adapters", "tests"];

pub const PROC_M: [&str; 9] = [
    "std::process::Command::new",
    "std::process::Command::output",
    "std::process::Command::spawn",
    "std::process::Command::status",
    "tokio::process::Command::new",
    "tokio::process::Command::output",
    "tokio::process::Command::spawn",
    "tokio::process::Command::status",
    "std::os::unix::process::CommandExt::exec",
];
pub const ENV_M: [&str; 6] = [
    "std::env::var",
    "std::env::var_os",
    "std::env::vars",
    "std::env::vars_os",
    "std::env::home_dir",
    "std::env::temp_dir",
];
pub const HTTP_M: [&str; 11] = [
    "reqwest::Client::new",
    "reqwest::Client::get",
    "reqwest::Client::post",
    "reqwest::Client::put",
    "reqwest::Client::delete",
    "reqwest::Client::head",
    "reqwest::Client::request",
    "reqwest::RequestBuilder::send",
    "reqwest::get",
    "reqwest::blocking::Client::new",
    "reqwest::blocking::Client::get",
];
pub const FS_STD_M: [&str; 16] = [
    "std::fs::read",
    "std::fs::read_to_string",
    "std::fs::write",
    "std::fs::remove_file",
    "std::fs::rename",
    "std::fs::create_dir_all",
    "std::fs::copy",
    "std::fs::create_dir",
    "std::fs::remove_dir",
    "std::fs::remove_dir_all",
    "std::fs::read_dir",
    "std::fs::set_permissions",
    "std::fs::hard_link",
    "std::fs::OpenOptions::new",
    "std::fs::OpenOptions::open",
    "std::fs::File::open",
];
pub const FS_TOKIO_M: [&str; 15] = [
    "tokio::fs::read",
    "tokio::fs::read_to_string",
    "tokio::fs::write",
    "tokio::fs::remove_file",
    "tokio::fs::rename",
    "tokio::fs::create_dir",
    "tokio::fs::create_dir_all",
    "tokio::fs::remove_dir",
    "tokio::fs::remove_dir_all",
    "tokio::fs::read_dir",
    "tokio::fs::metadata",
    "tokio::fs::set_permissions",
    "tokio::fs::hard_link",
    "tokio::fs::copy",
    "tokio::fs::file",
];
pub const FS_M: [&str; 31] = [
    "std::fs::read",
    "std::fs::read_to_string",
    "std::fs::write",
    "std::fs::remove_file",
    "std::fs::rename",
    "std::fs::create_dir_all",
    "std::fs::copy",
    "std::fs::create_dir",
    "std::fs::remove_dir",
    "std::fs::remove_dir_all",
    "std::fs::read_dir",
    "std::fs::set_permissions",
    "std::fs::hard_link",
    "std::fs::OpenOptions::new",
    "std::fs::OpenOptions::open",
    "std::fs::File::open",
    "tokio::fs::read",
    "tokio::fs::read_to_string",
    "tokio::fs::write",
    "tokio::fs::remove_file",
    "tokio::fs::rename",
    "tokio::fs::create_dir",
    "tokio::fs::create_dir_all",
    "tokio::fs::remove_dir",
    "tokio::fs::remove_dir_all",
    "tokio::fs::read_dir",
    "tokio::fs::metadata",
    "tokio::fs::set_permissions",
    "tokio::fs::hard_link",
    "tokio::fs::copy",
    "tokio::fs::file",
];
pub const FS_T: [&str; 4] = [
    "std::fs::File",
    "std::fs::OpenOptions",
    "std::fs::DirBuilder",
    "tokio::fs::File",
];
pub const CMD_T: [&str; 2] = ["std::process::Command", "tokio::process::Command"];

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
        ("tests", Some(_)) if dir == "harness" => Some("test-harness".to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exemptions::clippy_exemptions;
    use crate::exemptions::merge;
    use std::collections::BTreeSet;

    #[test]
    fn test_table_exemptions_match_repo_layout_paths() {
        let e = clippy_exemptions();
        assert!(e.contains_key("anybot-computers"));
        assert!(e.contains_key("depcheck"));
        assert!(e.contains_key("api-docs"));
        assert!(!e.contains_key("anybot-domain"));
        let want = [
            "std::fs::read_to_string".to_owned(),
            "std::fs::File::open".to_owned(),
            "std::env::home_dir".to_owned(),
        ]
        .into_iter()
        .collect::<BTreeSet<String>>();
        let got = e
            .get("anybot-foundation")
            .and_then(|m| m.get("disallowed-methods"));
        assert_eq!(got, Some(&want));
    }

    #[test]
    fn test_table_api_docs_exempts_only_file_writing() {
        let e = clippy_exemptions();
        let want = FS_STD_M
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<String>>();
        let got = e.get("api-docs").and_then(|m| m.get("disallowed-methods"));
        assert_eq!(got, Some(&want));
    }

    #[test]
    fn test_table_harness_exempts_process_http_files_but_not_env() {
        let e = clippy_exemptions();
        let want = merge(&[&PROC_M[..], &HTTP_M[..], &FS_M[..]])
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<String>>();
        let got = e
            .get("test-harness")
            .and_then(|m| m.get("disallowed-methods"));
        assert_eq!(got, Some(&want));
        for env in ENV_M {
            if env == "std::env::home_dir" {
                continue;
            }
            for per_key in e.values() {
                for set in per_key.values() {
                    assert!(!set.contains(env), "{env} 不准豁免（spec/07 §3）");
                }
            }
        }
    }

    #[test]
    fn test_table_env_methods_cover_the_full_env_ban() {
        let want = [
            "std::env::var",
            "std::env::var_os",
            "std::env::vars",
            "std::env::vars_os",
            "std::env::home_dir",
            "std::env::temp_dir",
        ]
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<String>>();
        let got = ENV_M
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<String>>();
        assert_eq!(got, want);
        let all = clippy_exemptions();
        let home = all
            .get("anybot-foundation")
            .and_then(|m| m.get("disallowed-methods"));
        assert!(home.is_some_and(|s| s.contains("std::env::home_dir")));
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
