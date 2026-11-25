use std::collections::BTreeMap;

use crate::codes::camel_case;
use crate::parse::{FtlEntry, duplicate_ids, entries_map, is_code_shape, parse_ftl};

#[must_use]
pub fn locale_pair_violations(en: &str, zh: &str) -> Vec<String> {
    let mut out = Vec::new();
    let en_entries = parse_ftl(en);
    let zh_entries = parse_ftl(zh);
    for (name, entries) in [("en", &en_entries), ("zh-TW", &zh_entries)] {
        for entry in entries {
            if !is_code_shape(&entry.id) {
                out.push(format!("{name}: bad code shape {}", entry.id));
            }
        }
        for dup in duplicate_ids(entries) {
            out.push(format!("{name}: duplicate code {dup}"));
        }
    }
    out.extend(id_set_violations(&en_entries, &zh_entries));
    out.extend(param_set_violations(
        &entries_map(&en_entries),
        &entries_map(&zh_entries),
    ));
    out
}

fn id_set_violations(en: &[FtlEntry], zh: &[FtlEntry]) -> Vec<String> {
    let en_ids = entries_map(en);
    let zh_ids = entries_map(zh);
    let mut out = Vec::new();
    for id in en_ids.keys() {
        if !zh_ids.contains_key(id) {
            out.push(format!("zh-TW is missing code {id}"));
        }
    }
    for id in zh_ids.keys() {
        if !en_ids.contains_key(id) {
            out.push(format!("en is missing code {id} (en is the baseline)"));
        }
    }
    out
}

fn param_set_violations(
    en: &BTreeMap<String, Vec<String>>,
    zh: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (id, en_params) in en {
        let Some(zh_params) = zh.get(id) else {
            continue;
        };
        if zh_params != en_params {
            out.push(format!(
                "code {id} params differ: en {en_params:?} zh-TW {zh_params:?}"
            ));
        }
    }
    out
}

#[must_use]
pub fn usage_violations(
    en: &str,
    rust_scan: &[String],
    ts_scan: &[String],
    all_scan: &[String],
) -> Vec<String> {
    let entries = parse_ftl(en);
    let mut out = Vec::new();
    for entry in &entries {
        if !code_referenced(entry, rust_scan, ts_scan, all_scan) {
            out.push(format!("code {} is not referenced by any source", entry.id));
        }
    }
    out.extend(unexisting_reference_violations(
        &entries, rust_scan, ts_scan,
    ));
    out
}

fn code_referenced(
    entry: &FtlEntry,
    rust_scan: &[String],
    ts_scan: &[String],
    all_scan: &[String],
) -> bool {
    let variant = camel_case(&entry.id);
    let rust_hit = rust_scan
        .iter()
        .any(|content| content.contains(&format!("MessageCode::{variant}")))
        || rust_scan.iter().any(|content| {
            content
                .lines()
                .filter(|line| is_const_code_line(line))
                .any(|line| {
                    quoted_code_candidates(line)
                        .iter()
                        .any(|q| q == &format!("{:?}", entry.id))
                })
        });
    let quoted = format!("{:?}", entry.id);
    let ts_hit = ts_scan.iter().any(|content| content.contains(&quoted));
    let literal_hit = all_scan.iter().any(|content| content.contains(&quoted));
    rust_hit || ts_hit || literal_hit
}

fn is_const_code_line(line: &str) -> bool {
    line.trim_start().starts_with("pub const") && line.contains("&str")
}

fn unexisting_reference_violations(
    entries: &[FtlEntry],
    rust_scan: &[String],
    ts_scan: &[String],
) -> Vec<String> {
    let known: Vec<String> = entries.iter().map(|e| format!("{:?}", e.id)).collect();
    let mut out = Vec::new();
    for content in rust_scan.iter().chain(ts_scan.iter()) {
        for line in content.lines().filter(|l| l.contains("MessageCode")) {
            for quoted in quoted_code_candidates(line) {
                if !known.contains(&quoted) {
                    out.push(format!(
                        "source references code {quoted} missing from en.ftl"
                    ));
                }
            }
        }
    }
    out
}

fn quoted_code_candidates(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = content.chars().collect();
    let has_dash = bytes.contains(&'-');
    let mut i = 0;
    while let Some(open) = bytes.get(i) {
        if *open != '"' {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => return out,
            };
            continue;
        }
        let Some(after_open) = i.checked_add(1) else {
            return out;
        };
        if bytes.get(after_open) == Some(&'"') {
            i = match after_open.checked_add(1) {
                Some(next) => next,
                None => return out,
            };
            continue;
        }
        let mut text = String::new();
        let mut j = after_open;
        while let Some(ch) = bytes.get(j) {
            if *ch == '"' || *ch == '\n' {
                break;
            }
            text.push(*ch);
            j = match j.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
        if bytes.get(j) == Some(&'"') && has_dash && is_code_shape(&text) {
            out.push(format!("\"{text}\""));
        }
        i = match j.checked_add(1) {
            Some(next) => next,
            None => return out,
        };
    }
    out
}

#[must_use]
pub fn generated_freshness_violations(
    entries: &[FtlEntry],
    committed_rust: &str,
    committed_ts: &str,
    expected_rust: &str,
    expected_ts: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    if committed_rust != expected_rust {
        out.push(format!(
            "generated Rust codes file is stale (regenerate with make locale-refresh; entries in en.ftl: {})",
            entries.len(),
        ));
    }
    if committed_ts != expected_ts {
        out.push(
            "generated TS codes file is stale (regenerate with make locale-refresh)".to_owned(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN_OK: &str = "a-code = Value { $port }\nb-code = Plain\n";
    const ZH_OK: &str = "a-code = Z { $port }\nb-code = Yan\n";

    #[test]
    fn test_locale_pair_violations_block_missing_code_in_zh() {
        let zh = "a-code = Z { $port }\n";
        let got = locale_pair_violations(EN_OK, zh);
        assert!(
            got.iter()
                .any(|v| v.contains("zh-TW is missing code b-code"))
        );
    }

    #[test]
    fn test_locale_pair_violations_block_param_mismatch() {
        let zh = "a-code = Z { $host }\nb-code = Yan\n";
        let got = locale_pair_violations(EN_OK, zh);
        assert!(got.iter().any(|v| v.contains("a-code params differ")));
    }

    #[test]
    fn test_locale_pair_violations_block_extra_code_in_zh_and_duplicates() {
        let zh = "a-code = Z { $port }\nb-code = Yan\nc-code = San\n";
        assert!(
            locale_pair_violations(EN_OK, zh)
                .iter()
                .any(|v| v.contains("en is missing code c-code"))
        );
        let dup_en = "a-code = One\na-code = Two\n";
        assert!(
            locale_pair_violations(dup_en, ZH_OK)
                .iter()
                .any(|v| v.contains("duplicate code a-code"))
        );
    }

    #[test]
    fn test_usage_violations_block_unused_and_missing_references() {
        let rust = vec!["use MessageCode::ACode;".to_owned()];
        let ts = vec!["export const x: MessageCode = \"b-code\";".to_owned()];
        let all = vec![rust.first().unwrap().clone(), ts.first().unwrap().clone()];
        let got = usage_violations(EN_OK, &rust, &ts, &all);
        assert!(got.is_empty());
        let ts_unused = vec!["export const x: MessageCode = \"b-code\";".to_owned()];
        let got2 = usage_violations(EN_OK, &[], &ts_unused, &ts_unused);
        assert!(got2.iter().any(|v| v.contains("a-code is not referenced")));
        let missing = vec!["const c: MessageCode = \"ghost-code\";".to_owned()];
        let got3 = usage_violations(EN_OK, &[], &missing, &missing);
        assert!(got3.iter().any(|v| v.contains("ghost-code")));
        let config = vec!["export default { environment: \"node\" };".to_owned()];
        let got4 = usage_violations(EN_OK, &[], &config, &config);
        assert!(got4.iter().all(|v| !v.contains("\"node\"")));
        let rust_ghost = vec!["let c = MessageCode::from_code(\"ghost-code\");".to_owned()];
        let got5 = usage_violations(EN_OK, &rust_ghost, &rust_ghost, &rust_ghost);
        assert!(got5.iter().any(|v| v.contains("ghost-code")));
        let rust_const = vec!["pub const AC: &str = \"a-code\";".to_owned()];
        let got6 = usage_violations(EN_OK, &rust_const, &[], &rust_const);
        assert!(got6.iter().all(|v| !v.contains("a-code is not referenced")));
    }

    #[test]
    fn test_generated_freshness_blocks_stale_files() {
        let entries = parse_ftl(EN_OK);
        let rust = crate::codes::rust_codes(&entries).expect("fixture codes format");
        let ts = crate::codes::ts_codes(&entries);
        assert!(generated_freshness_violations(&entries, &rust, &ts, &rust, &ts).is_empty());
        let stale = format!("{rust}\n// extra\n");
        let got = generated_freshness_violations(&entries, &stale, &ts, &rust, &ts);
        assert_eq!(got.len(), 1);
        assert!(got.first().is_some_and(|v| v.contains("stale")));
    }
}
