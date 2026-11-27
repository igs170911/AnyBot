use serde_json::json;

use crate::record_actions::{
    RESTORE_WINDOW_MS, is_chat_kind, is_hideable_action_kind, restore_window_open,
    scrub_chat_content,
};

#[test]
fn only_user_messages_and_model_answers_are_chat() {
    assert!(is_chat_kind("message"));
    assert!(is_chat_kind("model_answer"));
    assert!(!is_chat_kind("tool_completed"));
    assert!(!is_chat_kind("evidence_added"));
    assert!(!is_chat_kind("record_purge"));
}

#[test]
fn only_delete_and_clear_records_can_be_restored_or_purged() {
    assert!(is_hideable_action_kind("record_delete"));
    assert!(is_hideable_action_kind("record_purge"));
    assert!(!is_hideable_action_kind("record_restore"));
    assert!(!is_hideable_action_kind("message"));
}

#[test]
fn restore_is_open_at_thirty_days_and_closed_after() {
    let start = 1_000_i64;
    assert_eq!(
        restore_window_open(start, start + RESTORE_WINDOW_MS),
        Some(true)
    );
    assert_eq!(
        restore_window_open(start, start + RESTORE_WINDOW_MS + 1),
        Some(false)
    );
}

#[test]
fn restore_window_reports_none_when_the_clock_runs_backwards_past_the_limit() {
    assert_eq!(restore_window_open(i64::MAX, i64::MIN), None);
}

#[test]
fn scrubbing_a_message_drops_its_content_entirely() {
    let scrubbed = scrub_chat_content("message", &json!({"text": "hello"}));
    assert_eq!(scrubbed, serde_json::Value::Null);
}

#[test]
fn scrubbing_a_model_answer_keeps_structure_and_drops_text_and_thinking() {
    let content = json!({
        "text": "answer",
        "thinking": "reasoning",
        "tool_requests": [{"name": "read"}],
        "finish": "stop",
    });
    let scrubbed = scrub_chat_content("model_answer", &content);
    assert_eq!(
        scrubbed,
        json!({
            "text": null,
            "thinking": null,
            "tool_requests": [{"name": "read"}],
            "finish": "stop",
        })
    );
}

#[test]
fn scrubbing_a_model_answer_without_text_leaves_its_fields_alone() {
    let scrubbed = scrub_chat_content("model_answer", &json!({"finish": "stop"}));
    assert_eq!(scrubbed, json!({"finish": "stop"}));
}
