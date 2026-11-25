use crate::checks::locale::quoted_code_candidates;
use crate::parse::{FtlEntry, entries_map, parse_ftl};

fn const_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed
        .strip_prefix("pub const ")
        .or_else(|| trimmed.strip_prefix("const "))?;
    let name = rest.split(':').next().unwrap_or(rest);
    if name.starts_with("CODE_") {
        return Some(name.to_owned());
    }
    None
}

#[must_use]
pub fn string_code_const_violations(en: &str, rust_scan: &[String]) -> Vec<String> {
    let ids = entries_map(&parse_ftl(en));
    let mut out = Vec::new();
    for content in rust_scan {
        for line in content.lines() {
            let Some(name) = const_name(line) else {
                continue;
            };
            for quoted in quoted_code_candidates(line) {
                let id = quoted.trim_matches('"');
                if !ids.contains_key(id) {
                    out.push(format!(
                        "string constant {name} uses code {id} missing from en.ftl"
                    ));
                }
            }
        }
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
    locales: &str,
    committed_locales: &str,
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
    if committed_locales != locales {
        out.push(
            "generated TS locales file is stale (regenerate with make locale-refresh)".to_owned(),
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
    fn test_generated_freshness_blocks_stale_files() {
        let entries = parse_ftl(EN_OK);
        let rust = crate::codes::rust_codes(&entries).expect("fixture codes format");
        let ts = crate::codes::ts_codes(&entries);
        let locales = crate::codes::ts_locales(EN_OK, ZH_OK);
        assert!(
            generated_freshness_violations(&entries, &rust, &ts, &rust, &ts, &locales, &locales)
                .is_empty()
        );
        let stale = format!("{rust}\n// extra\n");
        let got =
            generated_freshness_violations(&entries, &stale, &ts, &rust, &ts, &locales, &locales);
        assert_eq!(got.len(), 1);
        assert!(got.first().is_some_and(|v| v.contains("stale")));
        let stale_locales = format!("{locales}\n// extra\n");
        let got2 = generated_freshness_violations(
            &entries,
            &rust,
            &ts,
            &rust,
            &ts,
            &locales,
            &stale_locales,
        );
        assert_eq!(got2.len(), 1);
        assert!(got2.first().is_some_and(|v| v.contains("locales")));
    }

    #[test]
    fn test_string_code_const_violations_block_planted_codes() {
        let rust = vec![
            "pub const CODE_SERVER_OK: &str = \"a-code\";\nconst CODE_PRIVATE: &str = \"b-code\";"
                .to_owned(),
        ];
        assert!(string_code_const_violations(EN_OK, &rust).is_empty());
        let planted = vec!["pub const CODE_PLANTED: &str = \"server-planted-missing\";".to_owned()];
        let got = string_code_const_violations(EN_OK, &planted);
        assert_eq!(got.len(), 1);
        assert!(got.first().is_some_and(|v| v.contains("CODE_PLANTED")));
        assert!(
            got.first()
                .is_some_and(|v| v.contains("server-planted-missing"))
        );
    }
}
