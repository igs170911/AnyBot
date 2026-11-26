#![cfg(test)]

use anybot_foundation::TimestampMs;

use crate::settings_merge::*;
use crate::settings_merge_shared::*;

#[test]
fn merge_prefers_the_later_edit() {
    let base = EPOCH.checked_add(1_000).unwrap();
    let mut state = SettingsState::default();
    state.apply(&edit(0, 1, 0, 100, base, 1));
    state.apply(&edit(1, 1, 0, 200, base, 2));
    let want = value_of_edit(&edit(1, 1, 0, 200, base, 2));
    assert_eq!(state.value_of(&"f1".to_owned()), Some(want));
    let mut first = SettingsState::default();
    first.apply(&edit(1, 1, 0, 200, base, 2));
    first.apply(&edit(0, 1, 0, 100, base, 1));
    let want = value_of_edit(&edit(1, 1, 0, 200, base, 2));
    assert_eq!(first.value_of(&"f1".to_owned()), Some(want));
}

#[test]
fn merge_respects_the_tie_break_rules() {
    let base = EPOCH.checked_add(1_000).unwrap();
    let by_device = edit(1, 2, 0, 500, base, 1);
    let by_seq = edit(1, 2, 0, 500, base, 9);
    let by_time = edit(0, 2, 0, 501, base, 0);
    let loser_device = edit(0, 2, 0, 500, base, 1);
    let loser_seq = edit(1, 2, 0, 500, base, 1);
    let loser_time = edit(0, 2, 0, 500, base, 9);

    let mut state = SettingsState::default();
    state.apply(&loser_device);
    state.apply(&by_device);
    let want = value_of_edit(&by_device);
    assert_eq!(state.value_of(&"f2".to_owned()), Some(want));

    let mut first = SettingsState::default();
    first.apply(&by_device);
    first.apply(&loser_device);
    let want = value_of_edit(&by_device);
    assert_eq!(first.value_of(&"f2".to_owned()), Some(want));

    let mut state = SettingsState::default();
    state.apply(&by_device);
    state.apply(&loser_seq);
    state.apply(&by_seq);
    let want = value_of_edit(&by_seq);
    assert_eq!(state.value_of(&"f2".to_owned()), Some(want));

    let mut first = SettingsState::default();
    first.apply(&by_seq);
    first.apply(&loser_seq);
    let want = value_of_edit(&by_seq);
    assert_eq!(first.value_of(&"f2".to_owned()), Some(want));

    let mut state = SettingsState::default();
    state.apply(&by_time);
    state.apply(&loser_time);
    let want = value_of_edit(&by_time);
    assert_eq!(state.value_of(&"f2".to_owned()), Some(want));

    assert!(SettingsState::wins(&by_device, &loser_device));
    assert!(!SettingsState::wins(&loser_device, &by_device));
    assert!(SettingsState::wins(&by_seq, &loser_seq));
    assert!(!SettingsState::wins(&loser_seq, &by_seq));
    assert!(SettingsState::wins(&by_time, &loser_time));
    assert!(!SettingsState::wins(&loser_time, &by_time));
}

#[test]
fn corrected_time_rejects_and_clamps() {
    let received = TimestampMs::from_millis(EPOCH + 500);
    let big = TimestampMs::from_millis(1u64 << 63);
    assert_eq!(
        corrected_time(big, 0, received),
        Err(CorrectedTimeError::Overflow)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(1_000), 0, big),
        Err(CorrectedTimeError::Overflow)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(u64::MAX), 1, received),
        Err(CorrectedTimeError::Overflow)
    );
    assert_eq!(
        corrected_time(
            TimestampMs::from_millis(i64::MAX.cast_unsigned()),
            1,
            received
        ),
        Err(CorrectedTimeError::Overflow)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(100), -500_000, received),
        Err(CorrectedTimeError::BeforeEpoch)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(EPOCH + 1_000), 0, received),
        Ok(EPOCH + 500)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(EPOCH + 500), 0, received),
        Ok(EPOCH + 500)
    );
    assert_eq!(
        corrected_time(
            TimestampMs::from_millis(EPOCH + 200),
            -1_700_000_000_200,
            received
        ),
        Ok(0)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(EPOCH + 500), -200, received),
        Ok(EPOCH + 300)
    );
}
