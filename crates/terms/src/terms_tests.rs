use std::collections::BTreeSet;

use proptest::prelude::*;

use crate::{TOKENIZER_VERSION, index_terms, is_exact_key, query_terms};

fn assert_has(terms: &[String], expected: &[&str]) {
    for want in expected {
        assert!(
            terms.iter().any(|item| item == want),
            "missing term {want:?} in {terms:?}"
        );
    }
}

fn coffee_expected_terms() -> [&'static str; 23] {
    [
        "\u{6211}",
        "\u{559c}",
        "\u{6b61}",
        "\u{990a}",
        "\u{8c93}",
        "\u{54aa}",
        "\u{548c}",
        "\u{559d}",
        "\u{7f8e}",
        "\u{5f0f}",
        "\u{5496}",
        "\u{5561}",
        "\u{6211}\u{559c}",
        "\u{559c}\u{6b61}",
        "\u{6b61}\u{990a}",
        "\u{990a}\u{8c93}",
        "\u{8c93}\u{54aa}",
        "\u{54aa}\u{548c}",
        "\u{548c}\u{559d}",
        "\u{559d}\u{7f8e}",
        "\u{7f8e}\u{5f0f}",
        "\u{5f0f}\u{5496}",
        "\u{5496}\u{5561}",
    ]
}

#[test]
fn index_terms_cjk_unigrams_bigrams_and_words() {
    assert_eq!(TOKENIZER_VERSION, 1);

    let coffee = "\u{6211}\u{559c}\u{6b61}\u{990a}\u{8c93}\u{54aa}\u{548c}\u{559d}\u{7f8e}\u{5f0f}\u{5496}\u{5561}";
    assert_has(&index_terms(coffee), &coffee_expected_terms());

    let weather = "\u{53f0}\u{5317}\u{5929}\u{6c23}\u{9810}\u{5831}\u{660e}\u{5929}\u{6703}\u{4e0b}\u{96e8}\u{55ce}";
    assert_has(
        &index_terms(weather),
        &[
            "\u{53f0}\u{5317}",
            "\u{5929}\u{6c23}",
            "\u{660e}\u{5929}",
            "\u{4e0b}\u{96e8}",
        ],
    );

    let trad_vs_simp = "\u{81fa}\u{7063}\u{548c}\u{53f0}\u{6e7e}\u{5404}\u{81ea}\u{5207}\u{5206}";
    assert_has(
        &index_terms(trad_vs_simp),
        &[
            "\u{81fa}\u{7063}",
            "\u{53f0}\u{6e7e}",
            "\u{5404}\u{81ea}",
            "\u{5207}\u{5206}",
        ],
    );
    assert!(
        !index_terms("\u{81fa}\u{7063}")
            .iter()
            .any(|t| t == "\u{53f0}\u{6e7e}")
    );
    assert!(
        !index_terms("\u{53f0}\u{6e7e}")
            .iter()
            .any(|t| t == "\u{81fa}\u{7063}")
    );
}

#[test]
fn index_terms_identifier_keeps_whole_and_parts() {
    let terms = index_terms("ERR_CONN_RESET CamelCase42 log/app/main.rs");
    assert_has(
        &terms,
        &[
            "err_conn_reset",
            "err",
            "conn",
            "reset",
            "camelcase42",
            "camel",
            "case",
            "42",
            "log/app/main.rs",
            "log",
            "app",
            "main",
            "rs",
        ],
    );

    assert!(is_exact_key("E1234"));
    assert!(is_exact_key("ERR_42"));
    assert!(is_exact_key("01900000-0000-7000-8000-000000000001"));
    assert!(!is_exact_key("hello"));
    assert!(!is_exact_key("12345"));
    assert!(!is_exact_key("log/app"));
    assert!(!is_exact_key("err conn"));

    assert!(index_terms("").is_empty());
    assert!(index_terms("   ...---///!!!   ").is_empty());
}

#[test]
fn index_terms_nfkc_lowercase() {
    let raw =
        "\u{ff25}\u{ff32}\u{ff32}\u{ff3f}\u{ff14}\u{ff12} \u{fb01}le \u{2460} \u{53f0}\u{7063}";
    let terms = index_terms(raw);
    assert_has(
        &terms,
        &["err_42", "err", "42", "file", "1", "\u{53f0}\u{7063}"],
    );
}

#[test]
fn query_terms_single_cjk_char() {
    let single = query_terms("\u{8c93}");
    assert_eq!(single.required_cjk, vec!["\u{8c93}"]);
    assert!(single.boost_words.is_empty());

    let multi = query_terms("\u{53f0}\u{5317}\u{5929}\u{6c23}");
    assert_eq!(
        multi.required_cjk,
        vec!["\u{53f0}\u{5317}", "\u{5317}\u{5929}", "\u{5929}\u{6c23}",]
    );
    assert!(multi.boost_words.iter().any(|w| w == "\u{53f0}\u{5317}"));

    let mixed = query_terms("ERR_42 log/app");
    assert_eq!(mixed.exact_keys, vec!["err_42"]);
    assert_eq!(
        mixed.identifier_groups,
        vec![
            vec!["err_42".to_owned(), "err".to_owned(), "42".to_owned()],
            vec!["log/app".to_owned(), "log".to_owned(), "app".to_owned()],
        ]
    );
    assert_eq!(mixed.trigram_phrases, vec!["err_42", "log/app"]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn index_terms_is_deterministic(text in ".*") {
        let first = index_terms(&text);
        let second = index_terms(&text);
        prop_assert_eq!(&first, &second);
        prop_assert_eq!(query_terms(&text), query_terms(&text));
        let unique: BTreeSet<&String> = first.iter().collect();
        prop_assert_eq!(unique.len(), first.len());
        prop_assert!(first.iter().all(|term| !term.is_empty() && !term.chars().any(char::is_uppercase)));
    }
}
