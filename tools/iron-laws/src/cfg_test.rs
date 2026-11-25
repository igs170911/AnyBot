use crate::comments::strip_rust_strings;

#[must_use]
pub fn violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        if !product_source(path) || !crate::table::has_extension(path, "rs") {
            continue;
        }
        out.extend(scan_file(path, content));
    }
    out
}

fn product_source(path: &str) -> bool {
    path.starts_with("crates/") || path.starts_with("apps/")
}

fn scan_file(path: &str, content: &str) -> Vec<String> {
    let stripped = strip_rust_strings(content);
    let mut out = Vec::new();
    for (index, line) in stripped.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("cfg(not(test))") {
            out.push(format!(
                "{path}:{} cfg(not(test)) is banned in product code",
                index.saturating_add(1)
            ));
        }
        if trimmed.contains("cfg!(test)") {
            out.push(format!(
                "{path}:{} cfg!(test) is banned in product code",
                index.saturating_add(1)
            ));
        }
        if !cfg_test_attr(trimmed) {
            continue;
        }
        let next = next_code_line(&stripped, index.saturating_add(1));
        if !cfg_target_allowed(next) {
            out.push(format!(
                "{path}:{} #[cfg(test)] is only allowed on mod and use",
                index.saturating_add(1)
            ));
        }
    }
    out
}

fn cfg_test_attr(trimmed: &str) -> bool {
    trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("#[cfg(test,")
}

fn next_code_line(content: &str, from: usize) -> &str {
    content
        .lines()
        .skip(from)
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .unwrap_or("")
}

fn cfg_target_allowed(next: &str) -> bool {
    next.starts_with("mod ")
        || next.starts_with("pub mod ")
        || next.starts_with("pub(crate) mod ")
        || next.starts_with("use ")
        || next.starts_with("pub use ")
        || next.starts_with("pub(crate) use ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_laws_cfg_not_test_is_blocked() {
        let content = "#[cfg(not(test))]\nfn hidden() {}\n";
        let v = violations(&[("crates/x/src/a.rs".to_owned(), content.to_owned())]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|row| row.contains("cfg(not(test))")));
    }

    #[test]
    fn test_iron_laws_cfg_test_on_fn_is_blocked() {
        let content = "#[cfg(test)]\nfn helper() {}\n";
        let v = violations(&[("apps/server/src/a.rs".to_owned(), content.to_owned())]);
        assert_eq!(v.len(), 1);
        assert!(
            v.first()
                .is_some_and(|row| row.contains("only allowed on mod and use"))
        );
    }

    #[test]
    fn test_iron_laws_cfg_macro_test_is_blocked() {
        let content = "fn hidden() -> bool { cfg!(test) }\n";
        let v = violations(&[("crates/x/src/a.rs".to_owned(), content.to_owned())]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|row| row.contains("cfg!(test)")));
    }

    #[test]
    fn test_iron_laws_cfg_test_on_mod_and_use_is_allowed() {
        let content =
            "#[cfg(test)]\nmod tests {\n    fn inside() {}\n}\n#[cfg(test)]\nuse other as _;\n";
        let v = violations(&[("crates/x/src/a.rs".to_owned(), content.to_owned())]);
        assert!(v.is_empty(), "{v:?}");
    }
}
