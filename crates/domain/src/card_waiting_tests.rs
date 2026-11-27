use serde_json::{Value, json};

use crate::card_waiting::{
    TaskRecordRef, card_is_waiting, card_relevant_kinds, is_card_relevant_kind,
};

fn shown(card: &str) -> Value {
    json!({"action": "shell", "card": card})
}

fn record<'a>(kind: &'a str, content: &'a Value) -> TaskRecordRef<'a> {
    TaskRecordRef { kind, content }
}

#[test]
fn the_card_that_was_shown_last_is_waiting() {
    let first = shown("card-1");
    assert!(card_is_waiting("card-1", &[record("card_shown", &first)]));
}

#[test]
fn a_card_nobody_showed_is_not_waiting() {
    assert!(!card_is_waiting("card-1", &[]));
    let other = shown("card-2");
    assert!(!card_is_waiting("card-1", &[record("card_shown", &other)]));
}

#[test]
fn the_next_card_replaces_the_previous_one() {
    let first = shown("card-1");
    let second = shown("card-2");
    let records = [record("card_shown", &first), record("card_shown", &second)];
    assert!(!card_is_waiting("card-1", &records));
    assert!(card_is_waiting("card-2", &records));
}

#[test]
fn stop_cancel_and_ended_tasks_leave_no_card_waiting() {
    let first = shown("card-1");
    let none = Value::Null;
    for ending in [
        "referee_stop",
        "referee_cancel",
        "task_stopped",
        "task_cancelled",
    ] {
        let records = [record("card_shown", &first), record(ending, &none)];
        assert!(!card_is_waiting("card-1", &records), "{ending}");
    }
}

#[test]
fn a_card_shown_after_a_stop_is_waiting_again() {
    let first = shown("card-1");
    let again = shown("card-2");
    let none = Value::Null;
    let records = [
        record("card_shown", &first),
        record("referee_stop", &none),
        record("card_shown", &again),
    ];
    assert!(card_is_waiting("card-2", &records));
}

#[test]
fn a_card_record_without_a_card_id_waits_for_nothing() {
    let broken = json!({"action": "shell"});
    assert!(!card_is_waiting("card-1", &[record("card_shown", &broken)]));
}

#[test]
fn only_card_and_ending_kinds_matter_for_the_waiting_check() {
    assert!(is_card_relevant_kind("card_shown"));
    assert!(is_card_relevant_kind("referee_stop"));
    assert!(!is_card_relevant_kind("model_answer"));
    assert!(!is_card_relevant_kind("referee_card_answer"));
}

#[test]
fn the_kind_list_for_the_store_matches_the_relevance_check() {
    let kinds = card_relevant_kinds();
    assert_eq!(kinds.len(), 5);
    assert!(kinds.iter().all(|k| is_card_relevant_kind(k)));
}
