use crate::table::{MAX_FUNCTION_LINES, MAX_SOURCE_LINES};

#[must_use]
pub fn file_line_violation(path: &str, content: &str) -> Option<String> {
    let n = content.lines().count();
    if n > MAX_SOURCE_LINES {
        return Some(format!(
            "{path}：檔案 {n} 行，超過上限 {MAX_SOURCE_LINES} 行（K76）"
        ));
    }
    None
}

#[must_use]
pub fn functions_over_limit_rust(content: &str, limit: usize) -> Vec<usize> {
    let stripped = crate::comments::strip_rust_strings(content);
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < stripped.lines().count() {
        let Some(span) = fn_span(&stripped, idx) else {
            idx = idx.saturating_add(1);
            continue;
        };
        if span.end.saturating_sub(span.start) > limit {
            out.push(span.start.saturating_add(1));
        }
        idx = span.end;
    }
    out
}

struct FnSpan {
    start: usize,
    end: usize,
}

fn fn_head(head: &str) -> bool {
    head.starts_with("fn ")
        || head.starts_with("pub fn ")
        || head.starts_with("async fn ")
        || head.starts_with("pub async fn ")
}

fn fn_span(content: &str, from: usize) -> Option<FnSpan> {
    let lines: Vec<&str> = content.lines().collect();
    let head = lines.get(from)?.trim();
    if !fn_head(head) {
        return None;
    }
    let mut depth = 0i32;
    let mut end = None;
    for (offset, line) in lines.iter().skip(from).enumerate() {
        depth = depth.saturating_add(depth_delta(line));
        let idx = from.saturating_add(offset);
        if depth == 0 && idx > from && line.contains('}') {
            end = Some(idx);
            break;
        }
    }
    Some(FnSpan {
        start: from,
        end: end.unwrap_or(lines.len()),
    })
}

fn depth_delta(line: &str) -> i32 {
    let open = i32::try_from(line.matches('{').count()).unwrap_or(i32::MAX);
    let close = i32::try_from(line.matches('}').count()).unwrap_or(i32::MAX);
    open.saturating_sub(close)
}

fn ts_open_line(t: &str) -> bool {
    t.ends_with(" => {")
        || t.ends_with(") {")
        || t.ends_with(") => {")
        || t.ends_with("function {")
        || (t.contains("function ") && t.ends_with('{'))
}

#[must_use]
pub fn functions_over_limit_ts(content: &str, limit: usize) -> Vec<usize> {
    let stripped = crate::comments::strip_ts_strings(content);
    let lines: Vec<&str> = stripped.lines().collect();
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if !ts_open_line(line.trim_end()) {
            continue;
        }
        let mut depth = 0i32;
        let mut end = idx;
        for (j, l) in lines.iter().enumerate().skip(idx) {
            depth = depth.saturating_add(depth_delta(l));
            if depth == 0 && j > idx && l.contains('}') {
                end = j;
                break;
            }
        }
        if end.saturating_sub(idx) > limit {
            out.push(idx.saturating_add(1));
        }
    }
    out
}

#[must_use]
pub fn violations(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, content) in files {
        if let Some(v) = file_line_violation(path, content) {
            out.push(v);
        }
        if crate::table::has_extension(path, "rs") {
            for line_no in functions_over_limit_rust(content, MAX_FUNCTION_LINES) {
                out.push(format!(
                    "{path}:{line_no} fn 長度超過上限 {MAX_FUNCTION_LINES} 行（K76）"
                ));
            }
        }
        if crate::table::has_extension(path, "ts") || crate::table::has_extension(path, "tsx") {
            for line_no in functions_over_limit_ts(content, MAX_FUNCTION_LINES) {
                out.push(format!(
                    "{path}:{line_no} 函式長度超過上限 {MAX_FUNCTION_LINES} 行（K76）"
                ));
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

    #[test]
    fn test_iron_laws_file_over_line_limit_is_blocked() {
        let big = "fn a() {}\n".repeat(MAX_SOURCE_LINES + 1);
        let v = violations(&[("crates/x/src/big.rs".to_owned(), big)]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("超過上限")));
    }

    #[test]
    fn test_iron_laws_rust_fn_over_line_limit_is_blocked() {
        let body = "    let x = 1;\n".repeat(MAX_FUNCTION_LINES + 1);
        let content = format!("fn long_fn() {{\n{body}}}\n");
        let v = violations(&[("crates/x/src/long.rs".to_owned(), content)]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("fn 長度超過上限")));
    }

    #[test]
    fn test_iron_laws_ts_fn_over_line_limit_is_blocked() {
        let body = "  const x = 1;\n".repeat(MAX_FUNCTION_LINES + 1);
        let content = format!("export function longFn(a: string) {{\n{body}}}\n");
        let v = violations(&[("web/ui/src/long.ts".to_owned(), content)]);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("函式長度超過上限")));
    }

    #[test]
    fn test_iron_laws_real_repo_files_and_functions_within_limits() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let files = source_files(&repo);
        assert!(!files.is_empty());
        assert_eq!(violations(&files), Vec::<String>::new());
    }
}
