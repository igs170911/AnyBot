pub use crate::banlists::{CMD_T, ENV_M, FS_M, FS_STD_M, FS_T, FS_TOKIO_M, HTTP_M, PROC_M, TIME_M};

pub const SCAN_DIRS: [&str; 6] = ["crates", "apps", "tools", "web", "adapters", "tests"];

pub const PACKAGE_SCAN_DIRS: [&str; 4] = ["src", "tests", "benches", "examples"];

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

pub const SKIP_DIR_NAMES: [&str; 9] = [
    "node_modules",
    "dist",
    "target",
    "gen",
    "build",
    "coverage",
    ".pnpm",
    "generated",
    "compilefail",
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
        ("tests", Some(_)) if dir == "app-test" => Some("anybot-app-test".to_owned()),
        ("tests", Some(_)) if dir == "live-model" => Some("live-model".to_owned()),
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
        assert!(e.contains_key("test-harness"));
        assert!(e.contains_key("live-model"));
        assert!(!e.contains_key("anybot-domain"));
        let want = [
            "std::fs::read_to_string".to_owned(),
            "std::fs::File::open".to_owned(),
            "std::env::home_dir".to_owned(),
            "time::OffsetDateTime::now_utc".to_owned(),
            "tokio::time::sleep".to_owned(),
            "tokio::time::sleep_until".to_owned(),
            "tokio::time::timeout".to_owned(),
            "tokio::time::Instant::now".to_owned(),
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
        let want = merge(&[&PROC_M[..], &HTTP_M[..], &FS_M[..], &TIME_M[..]])
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
                    assert!(
                        !set.contains(env),
                        "{env} must not be exempted (spec/07 section 3)"
                    );
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
        assert_eq!(
            package_of_path("tests/live-model/src/main.rs").as_deref(),
            Some("live-model")
        );
        assert_eq!(package_of_path("web/ui/src/shell.tsx"), None);
    }

    #[test]
    fn test_table_time_methods_cover_the_clock_ban() {
        let want = [
            "std::time::SystemTime::now",
            "std::time::Instant::now",
            "std::thread::sleep",
            "tokio::time::sleep",
            "tokio::time::sleep_until",
            "tokio::time::timeout",
            "tokio::time::interval",
            "tokio::time::Instant::now",
            "time::OffsetDateTime::now_utc",
        ]
        .into_iter()
        .map(String::from)
        .collect::<BTreeSet<String>>();
        let got = TIME_M
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<String>>();
        assert_eq!(got, want);
        let all = clippy_exemptions();
        let foundation = all
            .get("anybot-foundation")
            .and_then(|m| m.get("disallowed-methods"));
        assert!(foundation.is_some_and(|s| s.contains("time::OffsetDateTime::now_utc")));
        let harness = all
            .get("test-harness")
            .and_then(|m| m.get("disallowed-methods"));
        assert!(harness.is_some_and(|s| s.contains("std::thread::sleep")));
        for name in TIME_M {
            if name == "std::time::SystemTime::now" || name == "std::time::Instant::now" {
                let models = all
                    .get("anybot-models")
                    .and_then(|m| m.get("disallowed-methods"));
                assert!(
                    !models.is_some_and(|s| s.contains(name)),
                    "{name} must not be exempted outside clock owners"
                );
            }
        }
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
