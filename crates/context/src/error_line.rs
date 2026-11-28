pub const ERROR_PATTERNS: [&str; 8] = [
    "error",
    "Error:",
    "ERROR",
    "FAILED",
    "panicked",
    "Traceback",
    "Exception",
    "fatal:",
];

#[must_use]
pub fn find_error_line(content: &str, exit_code: Option<i64>) -> Option<usize> {
    let code = exit_code?;
    if code == 0 {
        return None;
    }
    let total = content.chars().count();
    let mut line_end = total;
    for line in content.rsplit('\n') {
        let line_chars = line.chars().count();
        let line_start = line_end.saturating_sub(line_chars);
        if ERROR_PATTERNS.iter().any(|pattern| line.contains(pattern)) {
            return Some(line_start);
        }
        line_end = line_start.saturating_sub(1);
    }
    None
}
