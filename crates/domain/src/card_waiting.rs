use serde_json::Value;

pub const CARD_SHOWN_KIND: &str = "card_shown";

pub const CARD_ENDING_KINDS: [&str; 4] = [
    "referee_stop",
    "referee_cancel",
    "task_stopped",
    "task_cancelled",
];

#[derive(Debug, Clone, Copy)]
pub struct TaskRecordRef<'a> {
    pub kind: &'a str,
    pub content: &'a Value,
}

#[must_use]
pub fn card_relevant_kinds() -> Vec<String> {
    let mut kinds = vec![CARD_SHOWN_KIND.to_owned()];
    kinds.extend(CARD_ENDING_KINDS.iter().map(|k| (*k).to_owned()));
    kinds
}

#[must_use]
pub fn is_card_relevant_kind(kind: &str) -> bool {
    kind == CARD_SHOWN_KIND || CARD_ENDING_KINDS.contains(&kind)
}

#[must_use]
pub fn card_is_waiting(card_id: &str, task_records_in_order: &[TaskRecordRef<'_>]) -> bool {
    let mut waiting: Option<&str> = None;
    for record in task_records_in_order {
        if record.kind == CARD_SHOWN_KIND {
            waiting = record.content.get("card").and_then(Value::as_str);
        } else if CARD_ENDING_KINDS.contains(&record.kind) {
            waiting = None;
        }
    }
    waiting == Some(card_id)
}
