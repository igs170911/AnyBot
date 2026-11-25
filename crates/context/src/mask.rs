use crate::input::StepRecord;
use crate::numbers::K114_2_ERROR_WINDOW_THIRDS;

#[must_use]
pub fn result_status(step: &StepRecord) -> &'static str {
    match step.outcome {
        crate::input::StepOutcomeKind::Completed => "ok",
        crate::input::StepOutcomeKind::Failed => "fail",
        crate::input::StepOutcomeKind::Unknown => "unknown",
        crate::input::StepOutcomeKind::Pending => "pending",
    }
}

#[must_use]
pub fn result_summary(step: &StepRecord) -> String {
    let exit = step
        .exit_code
        .map(|code| format!(" exit={code}"))
        .unwrap_or_default();
    format!(
        "tool={} args={} status={} size={}{} ref={}",
        step.tool,
        step.args_summary,
        result_status(step),
        step.content.chars().count(),
        exit,
        step.source_ref
    )
}

#[must_use]
pub fn masked_line(step: &StepRecord) -> String {
    format!("masked: {} ", result_summary(step))
}

#[must_use]
pub fn duplicate_line(step: &StepRecord, first_step: usize) -> String {
    format!(
        "same as step {} result ref={}",
        first_step.saturating_add(1),
        step.source_ref
    )
}

struct Window {
    head: usize,
    middle: usize,
    tail: usize,
}

fn windows(limit: usize, has_error: bool) -> Window {
    if has_error {
        let third = limit.saturating_div(K114_2_ERROR_WINDOW_THIRDS);
        return Window {
            head: third,
            middle: third,
            tail: third,
        };
    }
    Window {
        head: limit.saturating_div(2),
        middle: 0,
        tail: limit.saturating_div(2),
    }
}

fn take_chars(text: &str, skip: usize, count: usize) -> String {
    text.chars().skip(skip).take(count).collect()
}

fn elision(elided: usize, source_ref: &str) -> String {
    format!("[elided {elided} chars, full text ref={source_ref}] ")
}

#[must_use]
pub fn render_view(step: &StepRecord, limit: usize) -> String {
    let total = step.content.chars().count();
    if total <= limit {
        return step.content.clone();
    }
    let has_error = !step.error_at.is_empty();
    let w = windows(limit, has_error);
    let mut out = take_chars(&step.content, 0, w.head);
    out.push_str(&elision(
        total.saturating_sub(w.head.saturating_add(w.tail)),
        &step.source_ref,
    ));
    if has_error {
        out.push_str(&error_window(&step.content, &step.error_at, w.middle));
    }
    out.push_str(&take_chars(
        &step.content,
        total.saturating_sub(w.tail),
        w.tail,
    ));
    out
}

fn error_window(content: &str, error_at: &[usize], width: usize) -> String {
    let mut out = String::new();
    let half = width.saturating_div(2);
    for pos in error_at.iter().take(4) {
        let skip = pos.saturating_sub(half);
        out.push_str(&take_chars(content, skip, width));
        out.push_str(" [error@] ");
    }
    out
}
