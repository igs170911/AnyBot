#![cfg(test)]

use crate::settings_merge::*;
use crate::settings_merge_shared::*;

#[test]
fn merge_lists_item_by_item() {
    let base = EPOCH.checked_add(1_000).unwrap();
    let item0 = edit(2, 18, 0, 500, base, 0);
    let item0_late = edit(0, 18, 0, 600, base, 0);
    let item1 = edit(1, 19, 0, 500, base, 0);
    let item1_late = edit(2, 19, 0, 600, base, 0);
    let item2 = edit(0, 20, 0, 500, base, 0);
    let item2_late = edit(1, 20, 0, 600, base, 0);

    let mut state = SettingsState::default();
    state.apply_all(&[item0.clone(), item0_late.clone()]);
    state.apply_all(&[item1.clone(), item1_late.clone()]);
    state.apply_all(&[item2.clone(), item2_late.clone()]);
    assert_eq!(
        state.value_of(&"f18#i0".to_owned()),
        Some(value_of_edit(&item0_late))
    );
    assert_eq!(
        state.value_of(&"f19#i1".to_owned()),
        Some(value_of_edit(&item1_late))
    );
    assert_eq!(
        state.value_of(&"f20#i2".to_owned()),
        Some(value_of_edit(&item2_late))
    );

    let mut reverse = SettingsState::default();
    reverse.apply_all(&[
        item2_late.clone(),
        item2,
        item1_late.clone(),
        item1,
        item0_late.clone(),
        item0,
    ]);
    assert_eq!(
        reverse.value_of(&"f18#i0".to_owned()),
        Some(value_of_edit(&item0_late))
    );
    assert_eq!(
        reverse.value_of(&"f19#i1".to_owned()),
        Some(value_of_edit(&item1_late))
    );
    assert_eq!(
        reverse.value_of(&"f20#i2".to_owned()),
        Some(value_of_edit(&item2_late))
    );
}
