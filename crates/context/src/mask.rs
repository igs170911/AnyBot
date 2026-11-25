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

fn clip(start: usize, end: usize, total: usize) -> (usize, usize) {
    (start.min(total), end.min(total).max(start.min(total)))
}

fn merge_intervals(intervals: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut sorted: Vec<(usize, usize)> = intervals.to_vec();
    sorted.sort_unstable();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (start, end) in sorted {
        if end <= start {
            continue;
        }
        let hit = out.last_mut().and_then(|last| {
            if start <= last.1 {
                last.1 = end.max(last.1);
                return Some(true);
            }
            None
        });
        if hit.is_none() {
            out.push((start, end));
        }
    }
    out
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
    let mut intervals: Vec<(usize, usize)> = vec![(0, w.head)];
    if let Some(last) = step.error_at.iter().max() {
        let half = w.middle.saturating_div(2);
        intervals.push(clip(
            last.saturating_sub(half),
            last.saturating_add(half),
            total,
        ));
    }
    intervals.push(clip(total.saturating_sub(w.tail), total, total));
    let merged = merge_intervals(&intervals);
    let mut out = String::new();
    let mut cursor = 0usize;
    for (start, end) in merged {
        if start > cursor {
            out.push_str(&elision(start.saturating_sub(cursor), &step.source_ref));
        }
        out.push_str(&take_chars(&step.content, start, end.saturating_sub(start)));
        cursor = end;
    }
    if cursor < total {
        out.push_str(&elision(total.saturating_sub(cursor), &step.source_ref));
    }
    out
}
