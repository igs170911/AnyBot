use std::collections::{BTreeMap, BTreeSet};

pub const CODE_SOURCE_DELETED: &str = "context-source-deleted";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadQuery {
    Full,
    Find { needle: String },
    Window { from: usize, chars: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    Text {
        text: String,
    },
    Found {
        at: usize,
        context: String,
    },
    Window {
        from: usize,
        to: usize,
        total: usize,
        text: String,
    },
    NotFound,
    SourceDeleted,
}

pub struct RawStore {
    records: BTreeMap<String, String>,
    deleted: BTreeSet<String>,
}

impl RawStore {
    #[must_use]
    pub fn new(records: BTreeMap<String, String>, deleted: BTreeSet<String>) -> Self {
        Self { records, deleted }
    }

    #[must_use]
    pub fn is_deleted(&self, id: &str) -> bool {
        self.deleted.contains(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    #[must_use]
    pub fn chars(&self, id: &str) -> Option<usize> {
        self.records.get(id).map(|t| t.chars().count())
    }
}

#[must_use]
pub fn read_result(store: &RawStore, id: &str, query: &ReadQuery) -> ReadOutcome {
    if store.is_deleted(id) {
        return ReadOutcome::SourceDeleted;
    }
    let Some(text) = store.records.get(id) else {
        return ReadOutcome::NotFound;
    };
    match query {
        ReadQuery::Full => ReadOutcome::Text { text: text.clone() },
        ReadQuery::Find { needle } => read_find(text, needle),
        ReadQuery::Window { from, chars } => read_window(text, *from, *chars),
    }
}

fn char_slice(text: &str, start: usize, end: usize) -> String {
    text.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

const FIND_CONTEXT_CHARS: usize = 200;

fn read_find(text: &str, needle: &str) -> ReadOutcome {
    if needle.is_empty() {
        return ReadOutcome::NotFound;
    }
    let chars: Vec<char> = text.chars().collect();
    let pair: Vec<char> = needle.chars().collect();
    if pair.len() > chars.len() {
        return ReadOutcome::NotFound;
    }
    let limit = chars.len().saturating_sub(pair.len());
    for at in 0..=limit {
        if matches_at(&chars, &pair, at) {
            let ctx_start = at.saturating_sub(FIND_CONTEXT_CHARS);
            let ctx_end = at
                .saturating_add(pair.len())
                .saturating_add(FIND_CONTEXT_CHARS);
            return ReadOutcome::Found {
                at,
                context: char_slice(text, ctx_start, ctx_end),
            };
        }
    }
    ReadOutcome::NotFound
}

fn matches_at(chars: &[char], pair: &[char], at: usize) -> bool {
    chars.iter().skip(at).zip(pair.iter()).all(|(a, b)| a == b)
}

fn read_window(text: &str, from: usize, chars: usize) -> ReadOutcome {
    let total = text.chars().count();
    let to = total.min(from.saturating_add(chars));
    ReadOutcome::Window {
        from,
        to,
        total,
        text: char_slice(text, from, to),
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
