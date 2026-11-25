use crate::table::suppression_tokens;

fn normalize_line(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

#[must_use]
pub fn violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let tokens = suppression_tokens();
    for (path, content) in files {
        for (idx, line) in content.lines().enumerate() {
            let norm = normalize_line(line);
            for token in &tokens {
                if norm.contains(&token.to_lowercase()) {
                    out.push(format!(
                        "{path}:{} contains a check-suppression marker \"{token}\" (K75)",
                        idx.saturating_add(1)
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
    use crate::collect::source_files;
    use std::path::PathBuf;

    fn fixture(head: &str, tail: &str, rest: &str) -> String {
        format!("{head}{tail}{rest}")
    }

    #[test]
    fn test_iron_laws_allow_attribute_is_blocked() {
        let content = fixture("#[", "allow", "(clippy::unwrap_used)]\nfn main() {}\n");
        let v = violations(&[("crates/x/src/lib.rs".to_owned(), content)]);
        assert!(v.iter().any(|x| x.contains("K75")));
    }

    #[test]
    fn test_iron_laws_inner_allow_and_expect_are_blocked() {
        let a = fixture("#!", "[allow", "(unused)\nfn main() {}\n");
        let b = fixture("#!", "[expect", "(clippy::todo)\nfn main() {}\n");
        let c = fixture("#[", "expect", "(non_exhaustive)\nfn main() {}\n");
        let v = violations(&[
            ("crates/x/src/lib.rs".to_owned(), a),
            ("crates/y/src/lib.rs".to_owned(), b),
            ("crates/z/src/lib.rs".to_owned(), c),
        ]);
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn test_iron_laws_ts_and_biome_suppressions_are_blocked() {
        let content = fixture("const x = 1;\n", "", "")
            + &fixture("b", "iome-ignore", " lint/a\n")
            + &fixture("@ts-", "ignore", "\n")
            + &fixture("@ts-expect-", "error", "\n")
            + &fixture("es", "lint-disable", " next-line\n");
        let v = violations(&[("web/ui/src/x.ts".to_owned(), content)]);
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn test_iron_laws_real_repo_has_no_suppression_tokens() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let files = source_files(&repo);
        assert!(!files.is_empty());
        assert_eq!(violations(&files), Vec::<String>::new());
    }

    #[test]
    fn test_iron_laws_normalize_ignores_whitespace() {
        let spaced = format!("# [ {k} (x)]", k = "allow");
        let tight = format!("#[{k}(x)]", k = "allow");
        assert_eq!(normalize_line(&spaced), tight);
        assert_eq!(normalize_line("A  B\tC"), "abc");
    }
}
