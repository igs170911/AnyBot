use std::collections::BTreeSet;

pub fn spec_ids_from_md(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r"(?m)^\|\s*(T-[A-Za-z0-9_-]+)\s*\|")
        .map_err(|e| format!("規格編號樣式解析失敗：{e}"))?;
    Ok(re
        .captures_iter(content)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .collect())
}

pub fn code_ids_from_source(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(
        r"(?m)^\s*#\[[^\]]*\btest\b[^\]]*\]\s*\n(?:\s*(?://[^\n]*|#[^\n]*)\n)*\s*(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+t_([A-Za-z0-9_]+)\b",
    )
    .map_err(|e| format!("Rust 測試名稱樣式解析失敗：{e}"))?;
    Ok(re
        .captures_iter(content)
        .filter_map(|c| c.get(1))
        .map(|m| format!("T-{}", m.as_str().replace('_', "-")))
        .collect())
}

pub fn code_ids_from_ts_source(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r#"(?m)\b(?:test|it)\(\s*["'`](T-[A-Za-z0-9_-]+)["'`]"#)
        .map_err(|e| format!("TypeScript 測試標題樣式解析失敗：{e}"))?;
    Ok(re
        .captures_iter(content)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
        .collect())
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

#[must_use]
pub fn generate_baseline(spec_ids: &BTreeSet<String>, code_ids: &BTreeSet<String>) -> String {
    let fresh: BTreeSet<String> = spec_ids.difference(code_ids).cloned().collect();
    write_baseline(&fresh)
}

#[must_use]
pub fn check_baseline_against_ref(
    current: &BTreeSet<String>,
    ref_baseline: &BTreeSet<String>,
    new_spec_ids: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    for id in current.difference(ref_baseline) {
        if !new_spec_ids.contains(id) {
            violations.push(format!(
                "{id} 是基準檔新塞進來的（ref 的基準檔沒有它），但它不是這段變更裡規格新增的編號：基準檔只能變短，刪掉的測試不能把編號塞回來（M2 第 5 項）"
            ));
        }
    }
    violations
}

pub struct Report {
    pub violations: Vec<String>,
    pub total: usize,
    tested: usize,
    missing: usize,
}

impl Report {
    #[must_use]
    pub fn tested(&self) -> usize {
        self.tested
    }

    #[must_use]
    pub fn missing(&self) -> usize {
        self.missing
    }
}

#[must_use]
pub fn check(
    spec_ids: &BTreeSet<String>,
    code_ids: &BTreeSet<String>,
    baseline: Option<&BTreeSet<String>>,
) -> Report {
    let mut violations = Vec::new();
    let tested = spec_ids.intersection(code_ids).count();

    for id in code_ids.difference(spec_ids) {
        violations.push(format!("測試名稱標了規格沒有的編號：{id}"));
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

    for id in baseline_set.intersection(code_ids) {
        violations.push(format!(
            "{id} 已有測試，還留在基準檔：把它移除（基準檔只能變短）"
        ));
    }

    let not_in_baseline: BTreeSet<String> = spec_ids.difference(baseline_set).cloned().collect();
    for id in not_in_baseline.difference(code_ids) {
        violations.push(format!("規格新增編號 {id}：既不在基準檔、也沒有測試"));
    }

    Report {
        violations,
        total: spec_ids.len(),
        tested,
        missing: spec_ids.len().saturating_sub(tested),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
