//! depcheck · 讀 `cargo metadata`，照 `spec/07` §3 的表檢查依賴方向。
//!
//! 這是開發工具，不是產品程式碼：跑 cargo、讀 repo 的檔是它的本職，
//! 所以 clippy.toml 的禁用清單在這裡打開（depcheck 自己的規則只管 crates/ 和 apps/）。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::{BTreeMap, BTreeSet};

/// 內部套件准許依賴的內部套件（`spec/07` §3 的表；鍵用 Cargo 的 package 名）。
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
    // 執行檔：main 極簡，只接對應的函式庫套件（`spec/07` §2、§3 的「執行服務也是 API 的客戶端」）。
    add("app-server", &["server"]);
    add("app-runner-service", &["protocol"]);
    add("app-agent", &["sync"]);
    add("app-cli", &["foundation"]);
    // 檢查工具：不依賴任何產品套件。
    m.insert("depcheck".to_owned(), BTreeSet::new());
    m.insert("spec-trace".to_owned(), BTreeSet::new());
    m
}

/// domain 不准依賴的外部套件名：碰網路、檔案、資料庫、非同步執行環境的（`spec/07` §3）。
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

/// `allow(clippy::disallowed_methods)` / `disallowed_types` 准許出現的 package（`spec/07` §3 的四列＋開發工具本身）。
pub fn packages_allowed_exemptions() -> BTreeSet<String> {
    [
        "anybot-foundation",   // 讀環境變數（准許的套件，而且只一處）
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
        let Some(allow_set) = allowed.get(&pkg.name) else {
            violations.push(format!(
                "{}：depcheck 沒有這條依賴規則，請照 spec/07 §3 補進 allowed_internal_deps",
                pkg.name
            ));
            continue;
        };
        for dep in &pkg.dependencies {
            if dep.kind.as_deref().unwrap_or("normal") != "normal" {
                continue;
            }
            if internal.contains(&dep.name) && !allow_set.contains(&dep.name) {
                violations.push(format!(
                    "{} 依賴 {}：准許清單裡沒有（spec/07 §3）",
                    pkg.name, dep.name
                ));
            }
            if pkg.name == "anybot-domain" && forbidden_external_for_domain().contains(&dep.name) {
                violations.push(format!(
                    "anybot-domain 依賴 {}：domain 不准碰網路、檔案、資料庫、非同步（spec/07 §3）",
                    dep.name
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

/// 扫檔案內容，找 `allow(clippy::disallowed_methods)`／`allow(clippy::disallowed_types)`
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
}
