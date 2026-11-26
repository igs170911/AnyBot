#![cfg(test)]

use proptest::prelude::*;

use crate::settings_merge::*;
use crate::settings_merge_shared::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn merge_matches_independent_max_winner(tail in tail_strategy(300)) {
        let edits = [seed_edits(), tail].concat();
        let mut state = SettingsState::default();
        state.apply_all(&edits);
        let expected = expected_values(&edits);
        prop_assert!(!expected.is_empty());
        prop_assert!(list_key_count(&expected) >= LIST_ITEMS);
        for (key, want) in &expected {
            prop_assert_eq!(state.value_of(key), Some(want.clone()), "key {}", key.clone());
        }
    }

    #[test]
    fn merge_converges_under_random_orders_and_repeats(tail in tail_strategy(300)) {
        let edits = [seed_edits(), tail].concat();
        let size = edits.len();
        let orders = [
            shuffled_order(size, 11),
            shuffled_order(size, 22),
            shuffled_order(size, 33),
        ];
        let mut states = [
            SettingsState::default(),
            SettingsState::default(),
            SettingsState::default(),
        ];
        for (state, order) in states.iter_mut().zip(&orders) {
            let mut sent: Vec<FieldEdit> = Vec::new();
            for i in order {
                if let Some(e) = edits.get(*i) {
                    sent.push(e.clone());
                }
            }
            let half = sent.len() / 2;
            let mut repeated = Vec::new();
            for e in sent.iter().take(half) {
                repeated.push(e.clone());
            }
            sent.extend(repeated);
            state.apply_all(&sent);
        }
        let expected = expected_values(&edits);
        prop_assert!(list_key_count(&expected) >= LIST_ITEMS);
        for state in &states {
            for (key, want) in &expected {
                prop_assert_eq!(state.value_of(key), Some(want.clone()), "key {}", key.clone());
            }
        }
        prop_assert!(agreed(&states));
    }
}
