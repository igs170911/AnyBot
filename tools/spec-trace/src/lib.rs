//! spec-trace · 讀 `spec/10-behavior/[1-6]-*.md` 的測試編號表，掃程式碼的 `spec: T-…` 標記，
//! 用 `tools/spec-trace/baseline.txt` 記「還沒有測試的編號」。規則見 `docs/tasks/M1` 第 3 項、`spec/05` §2。
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
    let re = regex::Regex::new(r"(?m)^\|\s*(T-[A-Za-z0-9_-]+)\s*\|")
        .map_err(|e| format!("規格編號樣式解析失敗：{e}"))?;
    Ok(re.captures_iter(content).map(|c| c[1].to_owned()).collect())
}

/// 程式碼標記：`// spec: T-…` 或 `/// spec: T-…`。
///
/// # Errors
///
/// 樣式編譯失敗時回傳說明字串。
pub fn code_ids_from_source(content: &str) -> Result<BTreeSet<String>, String> {
    let re = regex::Regex::new(r"(?m)^[ \t]*(?://|///)[ \t]*spec:[ \t]*(T-[A-Za-z0-9_-]+)")
        .map_err(|e| format!("程式碼標記樣式解析失敗：{e}"))?;
    Ok(re.captures_iter(content).map(|c| c[1].to_owned()).collect())
}

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

/// 一次檢查的回報。
pub struct Report {
    pub violations: Vec<String>,
    /// 第一次跑：基準檔不存在，產生它。
    pub generated_baseline: Option<String>,
    pub total: usize,
    pub tested: usize,
    pub missing: usize,
}

/// 照關卡規則檢查。baseline=None 代表基準檔還沒有。
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
    let mut generated_baseline = None;

    // 規則 1：程式碼的編號必須都在規格裡。
    for id in code_ids.difference(spec_ids) {
        violations.push(format!("程式碼標了規格沒有的編號：{id}"));
    }

    let baseline_owned;
    let baseline_set = if let Some(b) = baseline {
        b
    } else {
        // 第一次產生：全部「還沒有測試的編號」放進去，之後只能變短。
        let fresh: BTreeSet<String> = spec_ids.difference(code_ids).cloned().collect();
        generated_baseline = Some(write_baseline(&fresh));
        baseline_owned = fresh;
        &baseline_owned
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

    let tested = spec_ids.intersection(code_ids).count();
    Report {
        violations,
        generated_baseline,
        total: spec_ids.len(),
        tested,
        missing: spec_ids.len().saturating_sub(tested),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn test_spec_trace_collects_table_ids() {
        // 範例用 `spec/10-behavior/README.md` 寫好的 chat-steer 測試表。
        // spec: T-chat-steer-1
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
    fn test_spec_trace_blocks_unknown_id() {
        // spec: T-chat-steer-1
        let r = check(&set(&["T-chat-steer-1"]), &set(&["T-nope-1"]), None);
        assert!(
            r.violations
                .iter()
                .any(|v| v.contains("程式碼標了規格沒有的編號：T-nope-1"))
        );
    }

    #[test]
    fn test_spec_trace_baseline_only_shrinks() {
        // spec: T-chat-steer-1
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
        // spec: T-chat-steer-1
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
    fn test_spec_trace_first_run_generates_baseline() {
        // spec: T-chat-steer-1
        let r = check(
            &set(&["T-chat-steer-1", "T-chat-steer-2"]),
            &set(&["T-chat-steer-1"]),
            None,
        );
        let generated = r.generated_baseline.unwrap();
        assert!(generated.contains("T-chat-steer-2"));
        assert!(!generated.contains("T-chat-steer-1"));
        assert!(r.violations.is_empty());
    }
}
