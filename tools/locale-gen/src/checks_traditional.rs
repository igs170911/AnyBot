pub const SIMPLIFIED_TABLE: [char; 49] = [
    '\u{7ED9}', '\u{542F}', '\u{4F17}', '\u{5199}', '\u{542C}', '\u{8BF4}', '\u{8BFB}', '\u{8F66}',
    '\u{957F}', '\u{95E8}', '\u{95EE}', '\u{5B9E}', '\u{521B}', '\u{636E}', '\u{56FE}', '\u{6837}',
    '\u{51B3}', '\u{8BE2}', '\u{6863}', '\u{5F53}', '\u{53D1}', '\u{53D8}', '\u{8BED}', '\u{8BCD}',
    '\u{8FD8}', '\u{4ECE}', '\u{8BA1}', '\u{8BBE}', '\u{5907}', '\u{8111}', '\u{5E93}', '\u{8F91}',
    '\u{95ED}', '\u{533A}', '\u{5355}', '\u{5360}', '\u{5382}', '\u{5386}', '\u{5F55}', '\u{6001}',
    '\u{603B}', '\u{62A5}', '\u{5BA1}', '\u{5BFC}', '\u{5C3D}', '\u{9500}', '\u{94FE}', '\u{7B80}',
    '\u{4F53}',
];

#[must_use = "the violations are consumed by the caller"]
pub fn simplified_violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        if !path.starts_with("locales/zh-TW") {
            continue;
        }
        for (line_no, line) in content.lines().enumerate() {
            for ch in line.chars() {
                if SIMPLIFIED_TABLE.contains(&ch) {
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
}
