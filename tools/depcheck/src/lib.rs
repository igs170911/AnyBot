//! depcheck · 讀 `cargo metadata` 檢查依賴方向（`spec/07` §3 的准許表），
//! 還有 domain 的外部依賴禁止（網路、檔案、資料庫、非同步）、
//! `apps/` 執行檔的「不准依賴」表（2026-11-24 補）。
//!
//! 也掃 source 檔：
//! * clippy.toml 的禁用（執行外部程式、發 HTTP、讀寫任意檔案）只准在指定套件，
//!   `allow(clippy::disallowed_methods|types)` 出现在准許以外的套件就擋。
//! * 讀環境變數在任何地方都不准（`spec/07` §3 2026-11-24 修正）：
//!   `crates/`、`apps/` 出現 `env::var`、`env::vars`、`env::var_os`、`env::vars_os`
//!   的文字就擋（套件層級的 allow 分不出類別，用文字檢查補）。
//!   `tools/` 的工程工具不受這條管；命令列參數可以讀。
//!
//! 這是開發工具，不在產品依賴圖裡：跑 cargo、讀 repo 是它的本職。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::{BTreeMap, BTreeSet};

/// 准許的內部依賴表（`spec/07` §3）。key = package 名，value = 准許的內部 package 名。
/// `models`、`tools`、`computers`、`mcp`「彼此不互相依賴」由准許清單（不含彼此）達成。
#[must_use]
pub fn allowed_internal_deps() -> BTreeMap<String, BTreeSet<String>> {
    let mut m = BTreeMap::new();
    let mut add = |pkg: &str, deps: &[&str]| {
        m.insert(
            format!("anybot-{pkg}"),
            deps.iter()
                .map(|d| format!("anybot-{d}"))
                .collect::<BTreeSet<_>>(),
        );
    };
    add("foundation", &[]);
    add("protocol", &["foundation"]);
    add("domain", &["foundation", "protocol"]);
    add("trust", &["foundation", "protocol", "domain"]);
    add("models", &["foundation", "protocol", "domain"]);
    add("tools", &["foundation", "protocol", "domain"]);
    add("computers", &["foundation", "protocol", "domain"]);
    add("mcp", &["foundation", "protocol", "domain"]);
    add("blob", &["foundation"]);
    add("bus", &["foundation"]);
    add("store-server", &["foundation", "protocol", "domain"]);
    add("store-client", &["foundation", "protocol", "domain"]);
    add("sync", &["foundation", "protocol", "store-client"]);
    add("channels", &["foundation", "protocol", "domain"]);
    // runner：上面全部，除了 store-server、server。
    add(
        "runner",
        &[
            "foundation",
            "protocol",
            "domain",
            "trust",
            "models",
            "tools",
            "mcp",
            "computers",
            "external-agents",
            "store-client",
            "sync",
            "bus",
            "blob",
            "channels",
        ],
    );
    add(
        "external-agents",
        &["foundation", "protocol", "trust", "mcp"],
    );
    // server：不准依賴 runner、models、tools、computers（`spec/02` A1）。
    add(
        "server",
        &[
            "foundation",
            "protocol",
            "domain",
            "store-server",
            "blob",
            "bus",
            "channels",
        ],
    );
    add(
        "migrate-old",
        &["foundation", "protocol", "domain", "store-client"],
    );
    // 檢查工具：不依賴任何產品套件。
    m.insert("depcheck".to_owned(), BTreeSet::new());
    m.insert("spec-trace".to_owned(), BTreeSet::new());
    m
}

/// 執行檔（`apps/`）的「不准依賴」表（`spec/07` §3 2026-11-24 補）。
/// key = apps/ 底下的目錄名。
#[must_use]
pub fn disallowed_app_deps(app: &str) -> Vec<&'static str> {
    match app {
        // 伺服器 API 不跑腦（A1）。
        "server" => vec![
            "runner",
            "models",
            "tools",
            "computers",
            "mcp",
            "external-agents",
        ],
        // 執行服務也是 API 的客戶端：不直接碰伺服器資料庫。
        "runner-service" | "agent" | "cli" => vec!["server", "store-server"],
        _ => vec![],
    }
}

/// domain 不准依賴的外部套件名：碰網路、檔案、資料庫、非同步執行環境的（`spec/07` §3）。
#[must_use]
pub fn forbidden_external_for_domain() -> BTreeSet<String> {
    [
        "tokio",
        "futures",
        "futures-util",
        "async-std",
        "sqlx",
        "reqwest",
        "hyper",
        "rustls",
        "tempfile",
        "notify",
        "walkdir",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// 准許打開 `allow(clippy::disallowed_methods|types)` 的 package。
/// 照 `spec/07` §3：執行外部程式→computers、發 HTTP→models/mcp/sync/channels/computers、
/// 讀寫任意檔案→computers/blob/store-client，加上 foundation 只准讀 `--bootstrap`
/// 啟動檔這一處（讀環境變數由上面的文字檢查另外擋，不靠 allow 分類別）。
/// `tools/` 是開發工具，不在產品依賴圖裡，也允許。
#[must_use]
pub fn packages_allowed_exemptions() -> BTreeSet<String> {
    [
        "anybot-foundation",   // 讀 --bootstrap 啟動檔（只這一處）
        "anybot-computers",    // 執行外部程式、發 HTTP、讀寫任意檔案
        "anybot-models",       // 發 HTTP
        "anybot-mcp",          // 發 HTTP
        "anybot-sync",         // 發 HTTP
        "anybot-channels",     // 發 HTTP
        "anybot-blob",         // 讀寫任意檔案
        "anybot-store-client", // 讀寫任意檔案
        "depcheck",            // 開發工具：跑 cargo、讀 repo（定義這些規則的套件本身）
        "spec-trace",          // 開發工具：讀 spec/ 和程式碼
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[derive(serde::Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
}

#[derive(serde::Deserialize)]
pub struct Package {
    pub name: String,
    /// None = workspace 內的 path 依賴（內部套件）。
    pub source: Option<String>,
    pub dependencies: Vec<Dependency>,
}

#[derive(serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    /// 只檢查 normal 依賴（dev/build 不構成產品依賴方向）。
    pub kind: Option<String>,
}

/// 檢查 metadata；回傳所有違規說明（空 = 全綠）。
#[must_use]
pub fn check_metadata_value(meta: &Metadata) -> Vec<String> {
    let mut violations = Vec::new();
    let internal: BTreeSet<String> = meta
        .packages
        .iter()
        .filter(|p| p.source.is_none())
        .map(|p| p.name.clone())
        .collect();
    let allowed = allowed_internal_deps();

    for pkg in &meta.packages {
        if pkg.source.is_some() {
            continue;
        }
        let deps: BTreeSet<String> = pkg
            .dependencies
            .iter()
            .filter(|d| d.kind.as_deref().unwrap_or("normal") == "normal")
            .map(|d| d.name.clone())
            .collect();
        if let Some(app_dir) = pkg.name.strip_prefix("anybot-app-") {
            // 執行檔：只加真的用到的；不准依賴表（spec/07 §3）。
            for dep in &deps {
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
            continue;
        }
        let Some(allow_set) = allowed.get(&pkg.name) else {
            violations.push(format!(
                "{}：depcheck 沒有這條依賴規則，請照 spec/07 §3 補進 allowed_internal_deps",
                pkg.name
            ));
            continue;
        };
        for dep in &deps {
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
    }
    violations
}

/// 讀環境變數在任何地方都不准（`spec/07` §3 2026-11-24 修正）。
/// 套件層級的 `allow` 分不出是哪一類禁用，所以做文字檢查：
/// `crates/`、`apps/` 的原始檔只要出現這四個寫法就一律擋
/// （`tools/` 的工程工具不受這條管；命令列參數 `env::args` 可以讀）。
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

/// 從相對於 repo 根的路徑推斷 package 名；推不出來回 None。
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

/// 掃檔案內容，找 `allow(clippy::disallowed_methods)`／`allow(clippy::disallowed_types)`
/// 出現在不准的套件 → 違規。
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

/// 檢查 cargo metadata 的 JSON 字串。
///
/// # Errors
///
/// JSON 解析失敗時回傳說明字串。
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

    #[test]
    fn test_depcheck_ok_for_allowed_edges() {
        // 准許的方向：server 依賴 store-server、sync 依賴 store-client。
        let meta = Metadata {
            packages: vec![
                pkg("anybot-store-server", &["anybot-foundation"]),
                pkg("anybot-store-client", &["anybot-foundation"]),
                pkg(
                    "anybot-server",
                    &["anybot-store-server", "anybot-foundation"],
                ),
                pkg("anybot-sync", &["anybot-store-client"]),
            ],
        };
        assert_eq!(check_metadata_value(&meta), Vec::<String>::new());
    }

    #[test]
    fn test_depcheck_blocks_reverse_and_banned_edges() {
        // 反向：foundation 依賴 protocol → 擋。
        // 表裡明禁：server 依賴 runner → 擋。domain 依賴 tokio → 擋。
        let meta = Metadata {
            packages: vec![
                pkg("anybot-foundation", &["anybot-protocol"]),
                pkg("anybot-domain", &["tokio"]),
                pkg("anybot-server", &["anybot-runner"]),
                pkg("anybot-protocol", &[]),
                pkg("anybot-runner", &[]),
            ],
        };
        let v = check_metadata_value(&meta);
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
    fn test_depcheck_blocks_unknown_package_and_bad_attr() {
        // 沒規則的內部套件 → 擋。
        let meta = Metadata {
            packages: vec![pkg("anybot-mystery", &["anybot-foundation"])],
        };
        assert!(check_metadata_value(&meta)[0].contains("anybot-mystery"));
        // allow(clippy::disallowed_methods) 出現在不准的套件 → 擋。
        let files = vec![
            (
                "crates/domain/src/lib.rs".to_owned(),
                "#![allow(clippy::disallowed_methods)]\n".to_owned(),
            ),
            (
                "crates/computers/src/lib.rs".to_owned(),
                "#![allow(clippy::disallowed_methods)]\n".to_owned(),
            ),
            (
                "apps/cli/src/main.rs".to_owned(),
                "// allow(clippy::disallowed_types) 在 apps 不准\n".to_owned(),
            ),
        ];
        let v = scan_exemption_attrs(&files);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|s| s.contains("crates/domain/src/lib.rs:1")));
        assert!(v.iter().any(|s| s.contains("apps/cli/src/main.rs:1")));
    }

    #[test]
    fn test_depcheck_app_dependency_table() {
        // 執行檔不准依賴表（spec/07 §3）：apps/server 依賴 runner → 擋；
        // apps/runner-service 依賴 server、store-server → 擋；正常的依賴不擋。
        let meta = Metadata {
            packages: vec![
                pkg("anybot-app-server", &["anybot-runner", "anybot-server"]),
                pkg(
                    "anybot-app-runner-service",
                    &["anybot-server", "anybot-store-server", "anybot-protocol"],
                ),
                pkg("anybot-app-agent", &["anybot-sync"]),
                pkg("anybot-app-cli", &["anybot-store-server"]),
            ],
        };
        let v = check_metadata_value(&meta);
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
        // 正常依賴不擋：runner-service→protocol、agent→sync、apps/server→server 本身。
        assert!(!v.iter().any(|s| s.contains("anybot-protocol")));
        assert!(!v.iter().any(|s| s.contains("anybot-sync")));
        assert!(
            !v.iter()
                .any(|s| s.contains("anybot-app-server 依賴 anybot-server"))
        );
    }

    #[test]
    fn test_depcheck_env_usage_text_check() {
        // 讀環境變數任何地方都不准：crates/、apps/ 出現文字就擋；tools/ 不掃。
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
                "// 命令列參數可以讀；這裡沒有禁用。\n".to_owned(),
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
}
