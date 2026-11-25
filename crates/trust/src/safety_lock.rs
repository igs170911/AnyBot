use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Unset,
    Openable,
    LockedOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyKey {
    AllowDirectMode,
    AllowLocalExecution,
    AllowBoxTrafficViaThisMac,
}

pub const POLICY_KEYS: [PolicyKey; 3] = [
    PolicyKey::AllowDirectMode,
    PolicyKey::AllowLocalExecution,
    PolicyKey::AllowBoxTrafficViaThisMac,
];

#[must_use]
pub fn parse_lock_value(raw: &str) -> LockState {
    let text = raw.trim().to_lowercase();
    if text.is_empty() {
        return LockState::Unset;
    }
    if text == "true" || text == "1" || text == "yes" {
        return LockState::Openable;
    }
    LockState::LockedOff
}

fn key_matches(name: &str, key: PolicyKey) -> bool {
    let expected = match key {
        PolicyKey::AllowDirectMode => "AllowDirectMode",
        PolicyKey::AllowLocalExecution => "AllowLocalExecution",
        PolicyKey::AllowBoxTrafficViaThisMac => "AllowBoxTrafficViaThisMac",
    };
    name.eq_ignore_ascii_case(expected)
}

#[must_use]
pub fn parse_policy(values: &BTreeMap<String, String>) -> BTreeMap<PolicyKey, LockState> {
    let mut out = BTreeMap::new();
    for key in POLICY_KEYS {
        let raw = values
            .iter()
            .find(|(name, _)| key_matches(name, key))
            .map(|(_, value)| value.as_str());
        let state = match raw {
            Some(text) => parse_lock_value(text),
            None => LockState::Unset,
        };
        out.insert(key, state);
    }
    out
}

#[must_use]
pub fn lock_blocks_direct_mode(states: &BTreeMap<PolicyKey, LockState>) -> bool {
    states
        .get(&PolicyKey::AllowDirectMode)
        .is_some_and(|state| *state == LockState::LockedOff)
}
