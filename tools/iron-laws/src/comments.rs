fn eat_until(out: &mut Vec<u8>, bytes: &[u8], i: &mut usize, close: u8, escape: u8) {
    while let Some(c) = bytes.get(*i) {
        if *c == close {
            out.push(close);
            *i = i.saturating_add(1);
            return;
        }
        if *c == escape && bytes.get(i.saturating_add(1)).is_some() {
            *i = i.saturating_add(1);
        }
        *i = i.saturating_add(1);
    }
}

fn scan_quotes(bytes: &[u8], quote: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while let Some(c) = bytes.get(i) {
        if *c == quote {
            out.push(*c);
            i = i.saturating_add(1);
            eat_until(&mut out, bytes, &mut i, quote, b'\\');
            continue;
        }
        out.push(*c);
        i = i.saturating_add(1);
    }
    out
}

fn suffix_hashes(bytes: &[u8], at: usize, hashes: usize) -> bool {
    (0..hashes).all(|k| bytes.get(at.saturating_add(k)).is_some_and(|b| *b == b'#'))
}

fn raw_string_scan(bytes: &[u8], out: &mut Vec<u8>, i: &mut usize) -> bool {
    let Some(c) = bytes.get(*i) else {
        return false;
    };
    if *c != b'r' && *c != b'b' {
        return false;
    }
    let mut j = i.saturating_add(1);
    if bytes.get(j).is_some_and(|b| *b == b'r') {
        j = j.saturating_add(1);
    }
    let mut hashes = 0usize;
    while bytes.get(j).is_some_and(|b| *b == b'#') {
        hashes = hashes.saturating_add(1);
        j = j.saturating_add(1);
    }
    if bytes.get(j).is_none_or(|b| *b != b'"') {
        return false;
    }
    j = j.saturating_add(1);
    while let Some(c) = bytes.get(j) {
        if *c == b'"' && suffix_hashes(bytes, j, hashes) {
            j = j.saturating_add(1).saturating_add(hashes);
            break;
        }
        j = j.saturating_add(1);
    }
    out.push(b'"');
    *i = j;
    true
}

fn char_literal_scan(bytes: &[u8], out: &mut Vec<u8>, i: &mut usize) -> bool {
    let mut j = i.saturating_add(1);
    while let Some(c) = bytes.get(j) {
        match c {
            b'\n' => return false,
            b'\\' => j = j.saturating_add(2),
            b'\'' => {
                out.push(b'\'');
                out.push(b'\'');
                *i = j.saturating_add(1);
                return true;
            }
            _ => j = j.saturating_add(1),
        }
    }
    false
}

#[must_use]
pub fn strip_rust_strings(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while let Some(c) = bytes.get(i) {
        let c = *c;
        if c == b'"' {
            out.push(c);
            i = i.saturating_add(1);
            eat_until(&mut out, bytes, &mut i, b'"', b'\\');
            continue;
        }
        if c == b'\'' && char_literal_scan(bytes, &mut out, &mut i) {
            continue;
        }
        if matches!(c, b'r' | b'b') && raw_string_scan(bytes, &mut out, &mut i) {
            continue;
        }
        out.push(c);
        i = i.saturating_add(1);
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[must_use]
pub fn strip_ts_strings(content: &str) -> String {
    let mut out = scan_quotes(content.as_bytes(), b'"');
    out = scan_quotes(&out, b'\'');
    out = scan_quotes(&out, b'`');
    String::from_utf8_lossy(&out).into_owned()
}

fn comment_lines(stripped: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (idx, line) in stripped.lines().enumerate() {
        if line.contains("//") || line.contains("/*") {
            out.push(idx.saturating_add(1));
        }
    }
    out
}

#[must_use]
pub fn rust_violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        if !crate::table::has_extension(path, "rs") {
            continue;
        }
        for line in comment_lines(&strip_rust_strings(content)) {
            out.push(format!(
                "{path}:{line} 有註解（K78：程式即註解，不准寫註解）"
            ));
        }
    }
    out
}

#[must_use]
pub fn violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = rust_violations(files);
    out.extend(ts_violations(files));
    out
}

#[must_use]
pub fn ts_violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        if !(crate::table::has_extension(path, "ts") || crate::table::has_extension(path, "tsx")) {
            continue;
        }
        for line in comment_lines(&strip_ts_strings(content)) {
            out.push(format!(
                "{path}:{line} 有註解（K78：程式即註解，不准寫註解）"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_laws_plain_line_comment_is_blocked() {
        let v = rust_violations(&[(
            "crates/x/src/lib.rs".to_owned(),
            "fn main() {}\n// 這是一行註解\n".to_owned(),
        )]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains(":2")));
    }

    #[test]
    fn test_iron_laws_doc_comments_are_blocked() {
        let v = rust_violations(&[(
            "crates/x/src/lib.rs".to_owned(),
            "//! 套件層註解\n/// 項目層註解\nfn main() {}\n".to_owned(),
        )]);
        assert_eq!(v.len(), 2);
        let t = ts_violations(&[(
            "web/x/src/a.tsx".to_owned(),
            "/** 檔首區塊 */\nexport const x = 1; // 行尾註解\n".to_owned(),
        )]);
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn test_iron_laws_text_inside_strings_is_not_a_comment() {
        let v = rust_violations(&[(
            "crates/x/src/lib.rs".to_owned(),
            "const s = \"// 不是註解 // 斜線\";\nconst r = r\"/* 不是區塊 */\";\n".to_owned(),
        )]);
        assert!(v.is_empty());
        let t = ts_violations(&[(
            "web/x/src/a.ts".to_owned(),
            "const u = \"http://localhost\";\nconst b = `a // b`;\n".to_owned(),
        )]);
        assert!(t.is_empty());
    }

    #[test]
    fn test_iron_laws_real_repo_has_no_comments() {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let files = crate::collect::source_files(&repo);
        assert!(!files.is_empty());
        assert_eq!(rust_violations(&files), Vec::<String>::new());
        assert_eq!(ts_violations(&files), Vec::<String>::new());
    }
}
