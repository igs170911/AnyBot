use std::collections::{BTreeMap, BTreeSet};

use crate::read::{RawStore, ReadOutcome, ReadQuery, read_result};

fn store_with(text: &str) -> RawStore {
    let mut records = BTreeMap::new();
    records.insert("r1".to_owned(), text.to_owned());
    RawStore::new(records, BTreeSet::new())
}

fn find(needle: &str, text: &str) -> (usize, String) {
    let store = store_with(text);
    if let ReadOutcome::Found { at, context } = read_result(
        &store,
        "r1",
        &ReadQuery::Find {
            needle: needle.to_owned(),
        },
    ) {
        return (at, context);
    }
    panic!("expected found")
}

fn window(from: usize, chars: usize, text: &str) -> (usize, usize, usize, String) {
    let store = store_with(text);
    if let ReadOutcome::Window {
        from,
        to,
        total,
        text,
    } = read_result(&store, "r1", &ReadQuery::Window { from, chars })
    {
        return (from, to, total, text);
    }
    panic!("expected window")
}

#[test]
fn read_find_and_window_return_the_asked_slice() {
    let text = format!("{}ERROR 42{}", "a".repeat(31_000), "b".repeat(18_992));
    let (at, context) = find("ERROR 42", &text);
    assert_eq!(at, 31_000);
    assert!(context.contains("ERROR 42"));
    assert!(context.chars().count() <= 440);
    let w = window(30_000, 2_000, &text);
    assert_eq!((w.0, w.1, w.2), (30_000, 32_000, 50_000));
    assert_eq!(w.3.chars().count(), 2_000);
}

#[test]
fn read_window_reports_from_to_and_total() {
    let text = "x".repeat(50_000);
    assert_eq!(
        window(30_000, 2_000, &text),
        (30_000, 32_000, 50_000, "x".repeat(2_000))
    );
}

#[test]
fn read_window_clips_past_the_end() {
    let text = "x".repeat(100);
    assert_eq!(window(50, 200, &text), (50, 100, 100, "x".repeat(50)));
}

#[test]
fn read_full_returns_stored_text() {
    let store = store_with("hello");
    assert_eq!(
        read_result(&store, "r1", &ReadQuery::Full),
        ReadOutcome::Text {
            text: "hello".to_owned()
        }
    );
}

#[test]
fn read_missing_id_reports_not_found() {
    let store = RawStore::new(BTreeMap::new(), BTreeSet::new());
    assert_eq!(
        read_result(&store, "r1", &ReadQuery::Full),
        ReadOutcome::NotFound
    );
}

#[test]
fn read_deleted_source_reports_deleted() {
    let mut records = BTreeMap::new();
    records.insert("r1".to_owned(), "hello".to_owned());
    let mut deleted = BTreeSet::new();
    deleted.insert("r1".to_owned());
    let store = RawStore::new(records, deleted);
    assert_eq!(
        read_result(
            &store,
            "r1",
            &ReadQuery::Find {
                needle: "hello".to_owned()
            }
        ),
        ReadOutcome::SourceDeleted
    );
}

#[test]
fn read_find_handles_multibyte_characters() {
    let text = format!(
        "{}\u{3057}\u{3073}{}",
        "\u{8a00}".repeat(10),
        "\u{8a00}".repeat(10)
    );
    let (at, context) = find("\u{3057}\u{3073}", &text);
    assert_eq!(at, 10);
    assert!(context.contains("\u{3057}\u{3073}"));
}
