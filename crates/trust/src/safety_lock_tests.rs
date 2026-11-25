use std::collections::BTreeMap;

use proptest::prelude::*;

use crate::safety_lock::*;

fn canonical_pair() -> BoxedStrategy<(String, LockState)> {
    prop_oneof![
        Just((String::new(), LockState::Unset)),
        Just(("true".to_owned(), LockState::Openable)),
        Just(("1".to_owned(), LockState::Openable)),
        Just(("yes".to_owned(), LockState::Openable)),
        Just(("false".to_owned(), LockState::LockedOff)),
        Just(("0".to_owned(), LockState::LockedOff)),
        Just(("no".to_owned(), LockState::LockedOff)),
        Just(("maybe".to_owned(), LockState::LockedOff)),
        Just(("TRUE!".to_owned(), LockState::LockedOff)),
    ]
    .boxed()
}

fn padded_canonical() -> BoxedStrategy<String> {
    (
        "[ ]{0,3}",
        prop_oneof![
            Just("true"),
            Just("1"),
            Just("YES"),
            Just("True"),
            Just("yEs")
        ],
        "[ ]{0,3}",
    )
        .prop_map(|(pre, core, post)| format!("{pre}{core}{post}"))
        .boxed()
}

fn policy_map(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("allowDirectMode".to_owned(), raw.to_owned());
    map.insert("ALLOWLOCALEXECUTION".to_owned(), raw.to_owned());
    map.insert("AllowBoxTrafficViaThisMac".to_owned(), raw.to_owned());
    map
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn t_safety_lock_1(raw in canonical_pair(), padded in padded_canonical()) {
        let states = parse_policy(&policy_map(&raw.0));
        prop_assert_eq!(states.len(), 3);
        for key in POLICY_KEYS {
            prop_assert_eq!(states.get(&key), Some(&raw.1));
        }
        prop_assert_eq!(parse_lock_value(&padded), LockState::Openable);
        let locked = parse_policy(&policy_map("maybe"));
        prop_assert!(lock_blocks_direct_mode(&locked));
        let openable = parse_policy(&policy_map(&padded));
        prop_assert!(!lock_blocks_direct_mode(&openable));
    }
}

#[test]
fn test_lock_missing_keys_are_unset() {
    let states = parse_policy(&BTreeMap::new());
    assert_eq!(states.len(), 3);
    assert!(states.values().all(|s| *s == LockState::Unset));
    for raw in ["false", "0", "no", "maybe", " 1x", "2", "yes?", "TRUE!"] {
        assert_eq!(parse_lock_value(raw), LockState::LockedOff, "raw {raw}");
    }
}
