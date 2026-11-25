use std::collections::BTreeMap;

use proptest::prelude::*;

use anybot_foundation::{DeviceId, TimestampMs};

use crate::settings_merge::*;

fn device_id(index: usize) -> DeviceId {
    DeviceId::parse(&format!("00000000-0000-4000-8000-{index:0>12}")).unwrap()
}

fn edit_strategy(devices: &[usize]) -> BoxedStrategy<FieldEdit> {
    (
        prop::sample::select(devices.to_vec()),
        0usize..20,
        (0usize..2usize),
        0u64..1_200_000,
        0i64..600_000,
    )
        .prop_map(move |(device_index, field, is_list, offset, time_shift)| {
            let device_time = TimestampMs::from_millis(
                1_700_000_000_000u64.saturating_add(u64::try_from(time_shift).unwrap_or(0)),
            );
            FieldEdit {
                field: format!("f{field}"),
                list_item: if is_list == 1 && field >= 18 {
                    Some(format!("i{}", field % 3))
                } else {
                    None
                },
                device: device_id(device_index),
                device_time,
                clock_offset_ms: offset,
                value: format!("v{device_index}-{field}"),
            }
        })
        .boxed()
}

fn expected_winners(edits: &[FieldEdit]) -> BTreeMap<String, FieldEdit> {
    let mut map: BTreeMap<String, FieldEdit> = BTreeMap::new();
    for edit in edits {
        match map.get(&edit.key()) {
            Some(existing) if SettingsState::wins(existing, edit) => {}
            _ => {
                map.insert(edit.key(), edit.clone());
            }
        }
    }
    map
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn t_sync_settings_4(edits in prop::collection::vec(edit_strategy(&[0, 1, 2]), 20..200)) {
        let mut states = [
            SettingsState::default(),
            SettingsState::default(),
            SettingsState::default(),
        ];
        for state in &mut states {
            state.apply_all(&edits);
        }
        prop_assert!(agreed(&states));
        for (key, winner) in expected_winners(&edits) {
            prop_assert_eq!(states[0].value_of(&key), Some(winner.value));
        }
    }
}
