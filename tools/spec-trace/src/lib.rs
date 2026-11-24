//! spec-trace · 讀 `spec/10-behavior/[1-6]-*.md` 的測試編號表，掃**產品程式碼**
//! （`crates/`、`apps/`、`tests/`、`web/`、`adapters/`）的 `spec: T-…` 標記
//! （只認註解，字串裡的不算；`tools/` 的工程工具不掃，測試不標編號、也不算覆蓋）。
//! 用 `tools/spec-trace/baseline.txt` 記「還沒有測試的編號」；缺失時關卡直接失敗，
//! 產生基準檔是明確指令 `make spec-baseline`。規則見 `docs/tasks/M1` 第 3 項、`spec/05` §2。
//!
//! 這是開發工具，不是產品程式碼：讀 repo 的檔是它的本職，clippy 禁用清單在這裡打開。

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::collections::BTreeSet;

/// 規格測試編號：表格「編號」欄裡 `T-` 開頭的（md 檔每列開頭 `| T-… |`）。
///
/// # Errors
///
/// 樣式編譯失敗時回傳說明字串。
pub fn spec_ids_from_md(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r"(?m)^\|\s*(T-[A-Za-z0-9_-]+)\s*\|")
        .map_err(|e| format!("規格編號樣式解析失敗：{e}"))?;
    Ok(re.captures_iter(content).map(|c| c[1].to_owned()).collect())
}

/// 程式碼標記：註解行 `// spec: T-…` 或 `/// spec: T-…`（字串裡出現的不算）。
///
/// # Errors
///
/// 樣式編譯失敗時回傳說明字串。
pub fn code_ids_from_source(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r"(?m)^[ \t]*(?://|///)[ \t]*spec:[ \t]*(T-[A-Za-z0-9_-]+)")
        .map_err(|e| format!("程式碼標記樣式解析失敗：{e}"))?;
    Ok(re.captures_iter(content).map(|c| c[1].to_owned()).collect())
}

#[must_use]
pub fn read_baseline(content: &str) -> BTreeSet<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

#[must_use]
pub fn write_baseline(ids: &BTreeSet<String>) -> String {
    let mut out = String::from("# spec-trace 基準檔：還沒有測試的規格編號（只能變短，不能變長）\n");
    for id in ids {
        out.push_str(id);
        out.push('\n');
    }
    out
}

/// 產生基準檔內容：全部「還沒有測試的編號」（給 `make spec-baseline` 用）。
#[must_use]
pub fn generate_baseline(spec_ids: &BTreeSet<String>, code_ids: &BTreeSet<String>) -> String {
    let fresh: BTreeSet<String> = spec_ids.difference(code_ids).cloned().collect();
    write_baseline(&fresh)
}

/// 一次檢查的回報。
pub struct Report {
    pub violations: Vec<String>,
    pub total: usize,
    pub tested: usize,
    pub missing: usize,
}

/// 照關卡規則檢查。基準檔不存在（baseline=None）→ 直接失敗，不自動產生
/// （產生是明確指令 `make spec-baseline` 的工作）。
/// 規則：
/// 1. 程式碼裡出現規格沒有的編號 → 擋。
/// 2. 基準檔裡的編號已經有測試了 → 擋（逼基準檔只能變短）。
/// 3. 規格多了新編號，既不在基準檔、也沒有測試 → 擋。
#[must_use]
pub fn check(
    spec_ids: &BTreeSet<String>,
    code_ids: &BTreeSet<String>,
    baseline: Option<&BTreeSet<String>>,
) -> Report {
    let mut violations = Vec::new();
    let tested = spec_ids.intersection(code_ids).count();

    // 規則 1：程式碼的編號必須都在規格裡。
    for id in code_ids.difference(spec_ids) {
        violations.push(format!("程式碼標了規格沒有的編號：{id}"));
    }

    let Some(baseline_set) = baseline else {
        violations.push(
            "基準檔 tools/spec-trace/baseline.txt 不存在：要重新產生請跑 make spec-baseline"
                .to_owned(),
        );
        return Report {
            violations,
            total: spec_ids.len(),
            tested,
            missing: spec_ids.len().saturating_sub(tested),
        };
    };

    // 規則 2：已經有測試的編號必須從基準檔移除。
    for id in baseline_set.intersection(code_ids) {
        violations.push(format!(
            "{id} 已有測試標記，還留在基準檔：把它移除（基準檔只能變短）"
        ));
    }

    // 規則 3：規格新增的編號要嘛進基準檔、要嘛有測試。
    let not_in_baseline: BTreeSet<String> = spec_ids.difference(baseline_set).cloned().collect();
    for id in not_in_baseline.difference(code_ids) {
        violations.push(format!("規格新增編號 {id}：既不在基準檔、也沒有測試標記"));
    }

    Report {
        violations,
        total: spec_ids.len(),
        tested,
        missing: spec_ids.len().saturating_sub(tested),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_spec_trace_collects_table_ids() {
        // 範例用 `spec/10-behavior/README.md` 寫好的 chat-steer 測試表。
        let md = "| 編號 | 種類 | 前提 | 步驟 | 預期 |\n\
                  |---|---|---|---|---|\n\
                  | T-chat-steer-1 | 端對端 | a | b | c |\n\
                  | T-chat-steer-2 | 整合 | a | b | c |\n\
                  內文提到的 T-chat-steer-9 在表格欄外，不進清單。\n";
        let ids = spec_ids_from_md(md).unwrap();
        assert!(ids.contains("T-chat-steer-1"));
        assert!(ids.contains("T-chat-steer-2"));
        assert!(!ids.contains("T-chat-steer-9"));
    }

    #[test]
    fn test_spec_trace_markers_only_in_comments() {
        // 只有註解裡的標記算；字串裡出現「spec: T-…」不算（避免自測資料變假覆蓋）。
        let src = "// spec: T-chat-steer-1\n\
                   let demo = \"// spec: T-chat-steer-2 在字串裡\";\n\
                   println!(\"x\"); // 一般註解\n";
        let ids = code_ids_from_source(src).unwrap();
        assert!(ids.contains("T-chat-steer-1"));
        assert!(!ids.contains("T-chat-steer-2"));
    }

    #[test]
    fn test_spec_trace_blocks_unknown_id() {
        let baseline = set(&["T-chat-steer-1"]);
        let r = check(
            &set(&["T-chat-steer-1"]),
            &set(&["T-nope-1"]),
            Some(&baseline),
        );
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("程式碼標了規格沒有的編號：T-nope-1"))
        );
    }

    #[test]
    fn test_spec_trace_baseline_only_shrinks() {
        let baseline = set(&["T-chat-steer-1", "T-chat-steer-2"]);
        let r = check(
            &set(&["T-chat-steer-1", "T-chat-steer-2"]),
            &set(&["T-chat-steer-1"]),
            Some(&baseline),
        );
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("T-chat-steer-1 已有測試標記，還留在基準檔"))
        );
        assert_eq!(r.total, 2);
        assert_eq!(r.tested, 1);
        assert_eq!(r.missing, 1);
    }

    #[test]
    fn test_spec_trace_new_id_must_be_tracked() {
        let baseline = set(&["T-chat-steer-1"]);
        let r = check(
            &set(&["T-chat-steer-1", "T-chat-steer-2"]),
            &set(&["T-chat-steer-1"]),
            Some(&baseline),
        );
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("規格新增編號 T-chat-steer-2"))
        );
    }

    #[test]
    fn test_spec_trace_missing_baseline_fails() {
        let r = check(
            &set(&["T-chat-steer-1", "T-chat-steer-2"]),
            &set(&["T-chat-steer-1"]),
            None,
        );
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("基準檔 tools/spec-trace/baseline.txt 不存在"))
        );
    }

    #[test]
    fn test_spec_trace_generate_baseline_excludes_tested() {
        let g = generate_baseline(
            &set(&["T-chat-steer-1", "T-chat-steer-2"]),
            &set(&["T-chat-steer-1"]),
        );
        assert!(g.contains("T-chat-steer-2"));
        assert!(!g.contains("T-chat-steer-1"));
    }
}
