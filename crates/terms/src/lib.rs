mod chars;

use std::collections::BTreeSet;
use std::sync::LazyLock;

use jieba_rs::Jieba;

pub const TOKENIZER_VERSION: u32 = 1;

static JIEBA: LazyLock<Jieba> = LazyLock::new(Jieba::new);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TermQuery {
    pub required_cjk: Vec<String>,
    pub boost_words: Vec<String>,
    pub identifier_groups: Vec<Vec<String>>,
    pub exact_keys: Vec<String>,
    pub trigram_phrases: Vec<String>,
}

impl TermQuery {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.required_cjk.is_empty()
            && self.boost_words.is_empty()
            && self.identifier_groups.is_empty()
            && self.exact_keys.is_empty()
            && self.trigram_phrases.is_empty()
    }
}

#[must_use]
pub fn index_terms(text: &str) -> Vec<String> {
    let normalized = chars::nfkc(text);
    let lowered = normalized.to_lowercase();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();

    for run in chars::cjk_runs(&lowered) {
        for &ch in &run {
            push_unique(&mut out, &mut seen, ch.to_string());
        }
        for pair in run.windows(2) {
            let bigram: String = pair.iter().collect();
            push_unique(&mut out, &mut seen, bigram);
        }
    }

    for token in JIEBA.cut(&lowered, false) {
        if token.word.chars().any(chars::is_cjk) {
            push_unique(&mut out, &mut seen, token.word.to_owned());
        }
    }

    for raw in chars::ident_tokens(&normalized) {
        push_unique(&mut out, &mut seen, raw.to_lowercase());
        for part in chars::split_ident_parts(&raw) {
            push_unique(&mut out, &mut seen, part);
        }
    }

    out
}

#[must_use]
pub fn query_terms(text: &str) -> TermQuery {
    let normalized = chars::nfkc(text);
    let lowered = normalized.to_lowercase();
    let mut query = TermQuery::default();
    collect_cjk_query(&lowered, &mut query);
    collect_ident_query(&normalized, &mut query);
    collect_trigram_phrases(&lowered, &mut query);
    query
}

#[must_use]
pub fn is_exact_key(token: &str) -> bool {
    let normalized = chars::nfkc(token);
    let trimmed = normalized
        .trim()
        .trim_matches(chars::is_ident_connector)
        .to_lowercase();
    if trimmed.is_empty() {
        return false;
    }
    if chars::is_uuid_shape(&trimmed) {
        return true;
    }
    let mut has_alpha = false;
    let mut has_digit = false;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphabetic() {
            has_alpha = true;
        } else if ch.is_ascii_digit() {
            has_digit = true;
        } else if !matches!(ch, '_' | '-') {
            return false;
        }
    }
    has_alpha && has_digit
}

fn collect_cjk_query(lowered: &str, query: &mut TermQuery) {
    let mut req_seen = BTreeSet::new();
    for run in chars::cjk_runs(lowered) {
        if let [single] = run.as_slice() {
            push_unique(&mut query.required_cjk, &mut req_seen, single.to_string());
        } else {
            for pair in run.windows(2) {
                let bigram: String = pair.iter().collect();
                push_unique(&mut query.required_cjk, &mut req_seen, bigram);
            }
        }
    }
    let mut boost_seen = BTreeSet::new();
    for token in JIEBA.cut(lowered, false) {
        if token.word.chars().any(chars::is_cjk) && token.word.chars().count() >= 2 {
            push_unique(
                &mut query.boost_words,
                &mut boost_seen,
                token.word.to_owned(),
            );
        }
    }
}

fn collect_ident_query(normalized: &str, query: &mut TermQuery) {
    let mut exact_seen = BTreeSet::new();
    let mut group_seen = BTreeSet::new();
    for raw in chars::ident_tokens(normalized) {
        let whole = raw.to_lowercase();
        if is_exact_key(&whole) {
            push_unique(&mut query.exact_keys, &mut exact_seen, whole.clone());
        }
        let mut group = Vec::new();
        let mut local_seen = BTreeSet::new();
        push_unique(&mut group, &mut local_seen, whole);
        for part in chars::split_ident_parts(&raw) {
            push_unique(&mut group, &mut local_seen, part);
        }
        if !group.is_empty() && group_seen.insert(group.clone()) {
            query.identifier_groups.push(group);
        }
    }
}

fn collect_trigram_phrases(lowered: &str, query: &mut TermQuery) {
    let mut seen = BTreeSet::new();
    for piece in lowered.split_whitespace() {
        let cleaned = piece.trim_matches(|ch: char| {
            !ch.is_alphanumeric() && !chars::is_cjk(ch) && !matches!(ch, '_' | '-' | '.' | '/')
        });
        let trimmed = cleaned.trim_matches(chars::is_ident_connector);
        if trimmed.chars().count() >= 3 {
            push_unique(&mut query.trigram_phrases, &mut seen, trimmed.to_owned());
        }
    }
}

fn push_unique(out: &mut Vec<String>, seen: &mut BTreeSet<String>, term: String) {
    if !term.is_empty() && seen.insert(term.clone()) {
        out.push(term);
    }
}

#[cfg(test)]
mod terms_tests;
