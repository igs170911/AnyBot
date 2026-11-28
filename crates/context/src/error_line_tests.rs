use crate::compile_tests_support as sup;
use crate::error_line::{ERROR_PATTERNS, find_error_line};
use crate::input::{StepOutcomeKind, StepRecord};
use crate::mask::render_view;
use crate::numbers::K114_2_DEFAULT_RESULT_CHARS;

fn make_doc(lines: usize, overrides: &[(usize, &str, &str)]) -> String {
    (1..=lines)
        .map(|n| {
            if let Some((_, prefix, body)) = overrides.iter().find(|(line, _, _)| *line == n) {
                format!("{prefix} {body: <13}\n")
            } else {
                format!("L{n:04} filler abcdef\n")
            }
        })
        .collect()
}

fn make_step(content: String, exit_code: Option<i64>, error_at: Option<usize>) -> StepRecord {
    StepRecord {
        seq: 1,
        call_id: sup::step_len(0, 0, "r").call_id,
        tool: "cmd".to_owned(),
        args_summary: "run".to_owned(),
        arguments: serde_json::json!({ "cmd": "run" }),
        level: anybot_protocol::ToolEffect::ReadOnly,
        unconfirmed: false,
        outcome: StepOutcomeKind::Failed,
        source_ref: "ref-el".to_owned(),
        content,
        exit_code,
        error_at,
    }
}

fn parse_segments(view: &str) -> Vec<String> {
    let mut parts = view.split("[elided ");
    let first = parts.next().unwrap_or("");
    let mut segments = vec![first.to_owned()];
    for part in parts {
        let (_, rest) = part
            .split_once(" chars, full text ref=ref-el] ")
            .expect("elision marker format");
        segments.push(rest.to_owned());
    }
    segments
}

fn check_pattern_case(pat: &str) {
    let content = make_doc(1_000, &[(500, "M0500", pat)]);
    let found = find_error_line(&content, Some(1));
    assert_eq!(found, Some(499usize.saturating_mul(20)));
    let step_auto = make_step(content.clone(), Some(1), None);
    let step_given = make_step(content, Some(1), found);
    let view = render_view(&step_given, K114_2_DEFAULT_RESULT_CHARS);
    assert_eq!(render_view(&step_auto, K114_2_DEFAULT_RESULT_CHARS), view);
    let segs = parse_segments(&view);
    assert_eq!(segs.len(), 3);
    assert!(segs.iter().all(|s| s.chars().count() <= 2_666));
    assert!(
        segs.first()
            .is_some_and(|s| s.starts_with("L0001 filler abcdef\n"))
    );
    assert!(
        segs.get(1)
            .is_some_and(|s| s.contains("L0499") && s.contains(pat) && s.contains("L0501"))
    );
    assert!(
        segs.get(2)
            .is_some_and(|s| s.ends_with("L1000 filler abcdef\n"))
    );
}

fn check_head_tail_only(content: String, exit_code: Option<i64>, hidden_marker: &str) {
    assert_eq!(find_error_line(&content, exit_code), None);
    let head_expected: String = content.chars().take(4_000).collect();
    let total = content.chars().count();
    let tail_expected: String = content.chars().skip(total.saturating_sub(4_000)).collect();
    let step = make_step(content, exit_code, None);
    let view = render_view(&step, K114_2_DEFAULT_RESULT_CHARS);
    let segs = parse_segments(&view);
    assert_eq!(segs.len(), 2);
    assert_eq!(
        segs.first().map(String::as_str),
        Some(head_expected.as_str())
    );
    assert_eq!(
        segs.get(1).map(String::as_str),
        Some(tail_expected.as_str())
    );
    assert!(!view.contains(hidden_marker));
}

#[test]
fn t_context_mask_6() {
    let spec_patterns = [
        "error",
        "Error:",
        "ERROR",
        "FAILED",
        "panicked",
        "Traceback",
        "Exception",
        "fatal:",
    ];
    assert_eq!(ERROR_PATTERNS.len(), 8);
    for pat in spec_patterns {
        check_pattern_case(pat);
    }

    let two_errors = make_doc(
        1_000,
        &[(300, "A0300", "Error: one"), (700, "B0700", "Error: two")],
    );
    let found = find_error_line(&two_errors, Some(1));
    assert_eq!(found, Some(699usize.saturating_mul(20)));
    let view = render_view(
        &make_step(two_errors, Some(1), found),
        K114_2_DEFAULT_RESULT_CHARS,
    );
    let segs = parse_segments(&view);
    assert_eq!(segs.len(), 3);
    assert!(segs.iter().all(|s| s.chars().count() <= 2_666));
    assert!(view.contains("B0700 Error: two"));
    assert!(!view.contains("A0300 Error: one"));

    let exit_zero = make_doc(1_000, &[(500, "M0500", "Error: ok")]);
    check_head_tail_only(exit_zero, Some(0), "M0500");

    let no_pattern = make_doc(1_000, &[(500, "M0500", "plain line")]);
    check_head_tail_only(no_pattern, Some(1), "M0500");

    let lower_failed = make_doc(1_000, &[(500, "M0500", "failed only")]);
    check_head_tail_only(lower_failed, Some(1), "M0500");
}

#[test]
fn error_line_ignores_success_and_missing_exit() {
    let mut body = String::new();
    for pat in ERROR_PATTERNS {
        body.push_str(pat);
        body.push('\n');
    }
    assert_eq!(find_error_line(&body, Some(0)), None);
    assert_eq!(find_error_line(&body, None), None);
    assert!(find_error_line(&body, Some(1)).is_some());
}

#[test]
fn find_error_line_counts_chars_not_bytes_and_collapses_same_line() {
    let content = "\u{4e2d}\u{6587}\nerror ERROR fatal:\nok\n";
    assert_eq!(find_error_line(content, Some(2)), Some(3));
}
