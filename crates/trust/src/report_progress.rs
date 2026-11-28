use anybot_foundation::CallId;
use anybot_protocol::{
    BudgetItem, KnownTaskEventKind, ProgressReport, ReasonPayload, TaskDetail, TaskEvent,
    TaskEventKind, ToolArgs,
};

use crate::message_codes_report::{CODE_REPORT_NEXT_RAISE_LIMIT, CODE_REPORT_STUCK_OVER_BUDGET};

const TARGET_KEYS: [&str; 4] = ["path", "line", "command", "url"];

#[must_use]
pub fn budget_item_name(item: BudgetItem) -> &'static str {
    match item {
        BudgetItem::Steps => "steps",
        BudgetItem::ToolCalls => "tool_calls",
        BudgetItem::Turns => "turns",
        BudgetItem::Elapsed => "elapsed",
        BudgetItem::Tokens => "tokens",
        BudgetItem::Cost => "cost",
        BudgetItem::Retries => "retries",
    }
}

#[must_use]
pub fn progress_of(events: &[TaskEvent], detail: Option<&TaskDetail>) -> Option<ProgressReport> {
    let Some(TaskDetail::OverBudget { item, used, limit }) = detail else {
        return None;
    };
    let targets = request_targets(events);
    let mut tally = Tally::default();
    for event in events {
        if let TaskEventKind::Known(kind) = &event.kind {
            tally.note(kind, &targets);
        }
    }
    let Tally { done, not_done, .. } = tally;
    let kind = budget_item_name(*item).to_owned();
    Some(ProgressReport {
        done,
        not_done,
        stuck: ReasonPayload {
            code: CODE_REPORT_STUCK_OVER_BUDGET.to_owned(),
            params: vec![
                ("used".to_owned(), used.to_string()),
                ("limit".to_owned(), limit.to_string()),
                ("kind".to_owned(), kind.clone()),
            ],
        },
        next: ReasonPayload {
            code: CODE_REPORT_NEXT_RAISE_LIMIT.to_owned(),
            params: vec![("kind".to_owned(), kind)],
        },
    })
}

#[derive(Default)]
struct Tally {
    done: Vec<String>,
    not_done: Vec<String>,
    failed: Vec<CallId>,
}

impl Tally {
    fn note(&mut self, kind: &KnownTaskEventKind, targets: &[(CallId, String)]) {
        if let KnownTaskEventKind::ToolFailed { call_id, .. } = kind {
            self.failed.push(*call_id);
        } else if let KnownTaskEventKind::ToolCompleted { call_id, .. } = kind
            && !self.failed.contains(call_id)
        {
            push_target(&mut self.done, targets, call_id);
        } else if let KnownTaskEventKind::ToolNotRun { call_id, .. } = kind {
            push_target(&mut self.not_done, targets, call_id);
        }
    }
}

fn push_target(out: &mut Vec<String>, targets: &[(CallId, String)], call_id: &CallId) {
    if let Some((_, label)) = targets.iter().find(|(id, _)| id == call_id) {
        out.push(label.clone());
    }
}

fn request_targets(events: &[TaskEvent]) -> Vec<(CallId, String)> {
    let mut out: Vec<(CallId, String)> = Vec::new();
    for event in events {
        let TaskEventKind::Known(KnownTaskEventKind::ModelAnswer { tool_requests, .. }) =
            &event.kind
        else {
            continue;
        };
        for request in tool_requests {
            out.push((request.call_id, target_label(&request.arguments)));
        }
    }
    out
}

fn target_label(arguments: &ToolArgs) -> String {
    match arguments {
        ToolArgs::Json { value } => TARGET_KEYS
            .iter()
            .find_map(|key| value.get(*key).and_then(|found| found.as_str()))
            .map_or_else(|| value.to_string(), str::to_owned),
        ToolArgs::Broken { raw } => raw.clone(),
    }
}
