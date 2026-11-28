use crate::message_codes::{
    CODE_TOOL_SHELL_FAILED, CODE_TOOL_SHELL_NO_OUTPUT, CODE_TOOL_SHELL_TIMED_OUT,
    CODE_TOOL_SHELL_TRIMMED,
};

pub const OUTPUT_EDGE_CHARS: usize = 6_000;
pub const SHELL_RESULT_CHARS: usize = 12_000;
pub const ERROR_PART_CHARS: usize = 4_000;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trimmed {
    pub head: String,
    pub tail: String,
    pub omitted: usize,
    pub error_window: Option<String>,
}

#[must_use]
pub fn trim_output(output: &str, exit_code: Option<i32>) -> Option<Trimmed> {
    let total = output.chars().count();
    if total <= SHELL_RESULT_CHARS {
        return None;
    }
    if let Some(err_pos) = find_error_line(output, exit_code) {
        return Some(three_parts(output, total, err_pos));
    }
    Some(two_edges(output, total))
}

fn two_edges(output: &str, total: usize) -> Trimmed {
    let head = take_first_chars(output, OUTPUT_EDGE_CHARS);
    let tail = take_last_chars(output, OUTPUT_EDGE_CHARS);
    Trimmed {
        head,
        tail,
        omitted: total.saturating_sub(OUTPUT_EDGE_CHARS.saturating_mul(2)),
        error_window: None,
    }
}

fn three_parts(output: &str, total: usize, err_pos: usize) -> Trimmed {
    let head_end = ERROR_PART_CHARS.min(total);
    let tail_start = total.saturating_sub(ERROR_PART_CHARS);
    let ideal_start = err_pos.saturating_sub(ERROR_PART_CHARS / 2);
    let max_start = tail_start.saturating_sub(ERROR_PART_CHARS).max(head_end);
    let mid_start = ideal_start.clamp(head_end, max_start);
    let mid_end = mid_start.saturating_add(ERROR_PART_CHARS).min(tail_start);
    let head = take_first_chars(output, head_end);
    let mid = slice_chars(output, mid_start, mid_end.saturating_sub(mid_start));
    let tail = take_last_chars(output, total.saturating_sub(tail_start));
    let kept = head_end
        .saturating_add(mid_end.saturating_sub(mid_start))
        .saturating_add(total.saturating_sub(tail_start));
    Trimmed {
        head,
        tail,
        omitted: total.saturating_sub(kept),
        error_window: Some(mid),
    }
}

fn find_error_line(output: &str, exit_code: Option<i32>) -> Option<usize> {
    if exit_code.is_none_or(|code| code == 0) {
        return None;
    }
    let mut offset = 0usize;
    let mut found = None;
    for segment in output.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if ERROR_PATTERNS.iter().any(|pat| line.contains(pat)) {
            found = Some(offset);
        }
        offset = offset.saturating_add(segment.chars().count());
    }
    found
}

fn take_first_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

fn take_last_chars(text: &str, count: usize) -> String {
    let rev: Vec<char> = text.chars().rev().take(count).collect();
    rev.into_iter().rev().collect()
}

fn slice_chars(text: &str, start: usize, count: usize) -> String {
    text.chars().skip(start).take(count).collect()
}

#[must_use]
pub fn shell_codes(exit_code: Option<i32>, output: &str) -> Vec<&'static str> {
    let mut codes = Vec::new();
    if output.is_empty() {
        codes.push(CODE_TOOL_SHELL_NO_OUTPUT);
    }
    if exit_code != Some(0) {
        codes.push(CODE_TOOL_SHELL_FAILED);
    }
    codes
}

#[must_use]
pub fn trim_code() -> &'static str {
    CODE_TOOL_SHELL_TRIMMED
}

#[must_use]
pub fn timeout_code() -> &'static str {
    CODE_TOOL_SHELL_TIMED_OUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_shell_output_keeps_the_head_and_the_tail_with_a_digest() {
        let output = "x".repeat(OUTPUT_EDGE_CHARS.saturating_mul(3));
        let trimmed = trim_output(&output, Some(0)).unwrap();
        assert_eq!(trimmed.head.chars().count(), OUTPUT_EDGE_CHARS);
        assert_eq!(trimmed.tail.chars().count(), OUTPUT_EDGE_CHARS);
        assert_eq!(trimmed.omitted, OUTPUT_EDGE_CHARS);
        assert_eq!(trimmed.error_window, None);
    }

    #[test]
    fn short_output_untouched() {
        assert_eq!(trim_output("short", Some(0)), None);
        let below = "y".repeat(11_999);
        assert_eq!(trim_output(&below, Some(0)), None);
        let exact = "y".repeat(SHELL_RESULT_CHARS);
        assert_eq!(trim_output(&exact, Some(0)), None);
        let over = "y".repeat(SHELL_RESULT_CHARS.saturating_add(1));
        assert!(trim_output(&over, Some(0)).is_some());
    }

    #[test]
    fn error_window_three_parts() {
        let output = format!("{}\nerror: boom\n{}", "a".repeat(8_000), "b".repeat(8_000));
        let trimmed = trim_output(&output, Some(2)).unwrap();
        let mid = trimmed.error_window.as_ref().unwrap();
        assert_eq!(trimmed.head.chars().count(), ERROR_PART_CHARS);
        assert_eq!(mid.chars().count(), ERROR_PART_CHARS);
        assert!(mid.contains("error: boom"));
        assert_eq!(trimmed.tail.chars().count(), ERROR_PART_CHARS);
        let kept =
            trimmed.head.chars().count() + mid.chars().count() + trimmed.tail.chars().count();
        assert!(kept <= SHELL_RESULT_CHARS);
    }

    #[test]
    fn exit_zero_ignores_error_words() {
        let output = format!("{}\nerror: boom\n{}", "a".repeat(8_000), "b".repeat(8_000));
        let zero = trim_output(&output, Some(0)).unwrap();
        assert_eq!(zero.error_window, None);
        assert_eq!(zero.head.chars().count(), OUTPUT_EDGE_CHARS);
        assert_eq!(zero.tail.chars().count(), OUTPUT_EDGE_CHARS);

        let missing = trim_output(&output, None).unwrap();
        assert_eq!(missing.error_window, None);

        let lower_failed = format!("{}\nfailed\n{}", "a".repeat(8_000), "b".repeat(8_000));
        assert_eq!(
            trim_output(&lower_failed, Some(1)).unwrap().error_window,
            None
        );
    }

    #[test]
    fn test_shell_result_codes_name_empty_output_and_failure() {
        assert_eq!(shell_codes(Some(0), ""), vec![CODE_TOOL_SHELL_NO_OUTPUT]);
        assert_eq!(shell_codes(Some(3), "noise"), vec![CODE_TOOL_SHELL_FAILED]);
        assert_eq!(shell_codes(Some(0), "noise"), Vec::<&str>::new());
    }
}
