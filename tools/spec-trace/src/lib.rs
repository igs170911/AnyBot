use std::collections::BTreeSet;

pub fn spec_ids_from_md(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r"(?m)^\|\s*(T-[A-Za-z0-9_-]+)\s*\|")
        .map_err(|e| format!("spec id pattern parse failed: {e}"))?;
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
    .map_err(|e| format!("rust test name pattern parse failed: {e}"))?;
    Ok(re
        .captures_iter(content)
        .filter_map(|c| c.get(1))
        .map(|m| format!("T-{}", m.as_str().replace('_', "-")))
        .collect())
}

pub fn code_ids_from_ts_source(content: &str) -> Result<BTreeSet<String>, String> {
    use regex::Regex;
    let re = Regex::new(r#"(?m)\b(?:test|it)\(\s*["'`](T-[A-Za-z0-9_-]+)(?:["'`]|:|\s)"#)
        .map_err(|e| format!("typescript test title pattern parse failed: {e}"))?;
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
    let mut out = String::from(
        "# spec-trace baseline: spec ids without a test yet (may only shrink, never grow)\n",
    );
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
    returned: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    for id in current.difference(ref_baseline) {
        if new_spec_ids.contains(id) || returned.contains(id) {
            continue;
        }
        violations.push(format!(
            "{id} was re-added to the baseline (the ref baseline lacks it) but it is not a spec id added by this change: the baseline may only shrink; deleted tests must not push ids back (M2 item 5)"
        ));
    }
    for id in returned {
        if !current.contains(id) {
            violations.push(format!(
                "{id} is listed in tools/spec-trace/returned.txt but is not in the baseline"
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
        violations.push(format!(
            "test name carries an id absent from the spec: {id}"
        ));
    }

    let Some(baseline_set) = baseline else {
        violations.push(
            "baseline tools/spec-trace/baseline.txt is missing: rerun make spec-baseline to regenerate it"
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
            "{id} already has a test and is still in the baseline: remove it (the baseline may only shrink)"
        ));
    }

    let not_in_baseline: BTreeSet<String> = spec_ids.difference(baseline_set).cloned().collect();
    for id in not_in_baseline.difference(code_ids) {
        violations.push(format!(
            "spec added id {id}: neither in the baseline nor covered by a test"
        ));
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
