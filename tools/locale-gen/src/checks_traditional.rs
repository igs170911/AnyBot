const TABLE_TEXT: &str = include_str!("../simplified-opencc.txt");

fn simplified_table() -> Vec<char> {
    let mut out = Vec::new();
    for line in TABLE_TEXT.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        push_escaped_line(&mut out, line);
    }
    out
}

fn push_escaped_line(out: &mut Vec<char>, line: &str) {
    for piece in line.split("\\u{").skip(1) {
        let Some((hex, _)) = piece.split_once('}') else {
            continue;
        };
        let Ok(code) = u32::from_str_radix(hex, 16) else {
            continue;
        };
        let Some(ch) = char::from_u32(code) else {
            continue;
        };
        if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
            out.push(ch);
        }
    }
}

#[must_use = "the violations are consumed by the caller"]
pub fn simplified_violations(files: &[(String, String)]) -> Vec<String> {
    let table = simplified_table();
    let mut out = Vec::new();
    for (path, content) in files {
        if !path.starts_with("locales/zh-TW") {
            continue;
        }
        for (line_no, line) in content.lines().enumerate() {
            for ch in line.chars() {
                if table.contains(&ch) {
                    let first_line = line_no.checked_add(1).unwrap_or_default();
                    out.push(format!(
                        "{path}:{first_line}: simplified character U+{code:04X} is not allowed in zh-TW (spec/09 K91)",
                        code = u32::from(ch),
                    ));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplified_gate_blocks_zh_tw_files() {
        let files = vec![(
            "locales/zh-TW.ftl".to_owned(),
            "a = 1 \u{7ED9}\n".to_owned(),
        )];
        let got = simplified_violations(&files);
        assert!(got.iter().any(|v| v.contains("U+7ED9")), "got: {got:?}");
        assert_eq!(got.len(), 1, "got: {got:?}");
    }

    #[test]
    fn test_simplified_gate_ignores_other_paths_and_clean_text() {
        let files = vec![
            (
                "locales/en.ftl".to_owned(),
                "a = Share it with you\n".to_owned(),
            ),
            ("spec/README.md".to_owned(), "b \u{7ED9} c\n".to_owned()),
            (
                "locales/zh-TW.ftl".to_owned(),
                "a = 1 \u{7D66}\n".to_owned(),
            ),
        ];
        assert!(simplified_violations(&files).is_empty());
    }

    #[test]
    fn test_simplified_gate_accepts_current_zh_tw_source() {
        let source = include_str!("../../../locales/zh-TW.ftl");
        let files = vec![("locales/zh-TW.ftl".to_owned(), source.to_owned())];
        assert!(simplified_violations(&files).is_empty());
    }

    #[test]
    fn test_simplified_table_covers_common_simplified_forms() {
        let table = simplified_table();
        for ch in [
            '\u{8FD9}', '\u{4E2A}', '\u{4EEC}', '\u{4E3A}', '\u{6765}', '\u{65F6}', '\u{8BF7}',
            '\u{7801}', '\u{53F7}', '\u{65E0}', '\u{9519}', '\u{8BEF}',
        ] {
            assert!(table.contains(&ch), "missing {ch}");
        }
        assert!(table.len() > 200, "table has {}", table.len());
    }
}
