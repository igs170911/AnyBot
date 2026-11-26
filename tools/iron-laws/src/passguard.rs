use crate::collect;

pub const PASS_BUILD_TOKENS: [&str; 3] = ["Pass::issue", "Pass {", "Pass{"];

#[must_use]
pub fn violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for (path, content) in files {
        if !path.to_ascii_lowercase().ends_with(".rs") || path.starts_with("crates/trust/") {
            continue;
        }
        let text = crate::comments::strip_rust_strings(content);
        for token in PASS_BUILD_TOKENS {
            if text.contains(token) {
                out.push(format!(
                    "{path} builds a pass token ({token}): only crates/trust may issue passes (spec/07 section 3)",
                ));
            }
        }
    }
    out
}

#[must_use]
pub fn violations_at_repo(repo: &std::path::Path) -> Vec<String> {
    violations(&collect::source_files(repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_laws_pass_construction_outside_trust_is_blocked() {
        let files = vec![
            (
                "crates/tools/src/lib.rs".to_owned(),
                "let p = Pass::issue(k, g, t);".to_owned(),
            ),
            (
                "crates/computers/src/lib.rs".to_owned(),
                "Pass { key, gate, issued_at }".to_owned(),
            ),
            (
                "crates/trust/src/pass.rs".to_owned(),
                "Pass { key, gate, issued_at }".to_owned(),
            ),
        ];
        let got = violations(&files);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(
            got.iter()
                .all(|v| v.contains("only crates/trust may issue passes"))
        );
    }

    #[test]
    fn test_iron_laws_pass_token_inside_a_string_literal_is_ignored() {
        let files = vec![(
            "crates/tools/src/lib.rs".to_owned(),
            "let note = \"Pass { key }\".to_owned();".to_owned(),
        )];
        assert_eq!(violations(&files), Vec::<String>::new());
    }

    #[test]
    fn test_iron_laws_real_repo_builds_no_passes_outside_trust() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(violations_at_repo(&repo), Vec::<String>::new());
    }
}
