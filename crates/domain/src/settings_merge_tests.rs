use std::collections::BTreeMap;

use proptest::prelude::*;

use anybot_foundation::{DeviceId, TimestampMs};

use crate::settings_merge::*;

const EPOCH: u64 = 1_700_000_000_000;

fn device_id(index: usize) -> DeviceId {
    DeviceId::parse(&format!("00000000-0000-4000-8000-{index:0>12}")).unwrap()
}

fn edit(
    device_index: usize,
    field: usize,
    offset: i64,
    shift: u64,
    received: u64,
    seq: u64,
) -> FieldEdit {
    FieldEdit {
        field: format!("f{field}"),
        list_item: if field >= 18 {
            Some(format!("i{}", field % 3))
        } else {
            None
        },
        device: device_id(device_index),
        device_time: TimestampMs::from_millis(EPOCH.saturating_add(shift)),
        clock_offset_ms: offset,
        received_ms: TimestampMs::from_millis(received),
        stream_seq: seq,
        value: format!("v{device_index}-{shift}-{seq}"),
    }
}

fn value_of_edit(edit: &FieldEdit) -> String {
    let shift = edit.device_time.millis().saturating_sub(EPOCH);
    let name = edit.device.as_str();
    let tail = name.rsplit('-').next().unwrap_or("0");
    let index: usize = tail.parse().unwrap_or(0);
    format!("v{index}-{shift}-{seq}", seq = edit.stream_seq)
}

fn seed_edits() -> Vec<FieldEdit> {
    let mut out = Vec::new();
    for field in 0..22 {
        for device in 0..3usize {
            out.push(edit(
                device,
                field,
                0,
                10,
                EPOCH.saturating_add(20),
                u64::try_from(device).unwrap_or(0),
            ));
        }
    }
    out
}

fn full_shuffle(keys: &[usize]) -> Vec<usize> {
    let mut pairs: Vec<(usize, usize)> = keys.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    pairs.sort_unstable();
    pairs.iter().map(|(_, i)| *i).collect()
}

fn edit_strategy(devices: &[usize]) -> BoxedStrategy<FieldEdit> {
    (
        prop::sample::select(devices.to_vec()),
        0usize..22,
        -600_000i64..600_000,
        0u64..1_200_000,
        EPOCH..EPOCH.saturating_add(1_200_000),
        0u64..1_000,
    )
        .prop_map(|(device_index, field, offset, shift, received, seq)| {
            edit(device_index, field, offset, shift, received, seq)
        })
        .boxed()
}

fn model_corrected(edit: &FieldEdit) -> u64 {
    let shift = edit
        .device_time
        .millis()
        .saturating_add_signed(edit.clock_offset_ms);
    shift.min(edit.received_ms.millis())
}

fn model_sort_key(edit: &FieldEdit) -> (u64, String, u64) {
    (
        model_corrected(edit),
        edit.device.as_str().clone(),
        edit.stream_seq,
    )
}

fn expected_values(edits: &[FieldEdit]) -> BTreeMap<String, String> {
    let mut best: BTreeMap<String, (u64, String, u64)> = BTreeMap::new();
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    for edit in edits {
        let key = model_sort_key(edit);
        match best.get(&edit.key()) {
            Some(top) if *top >= key => {}
            _ => {
                best.insert(edit.key(), key);
                values.insert(edit.key(), value_of_edit(edit));
            }
        }
    }
    values
}

fn list_key_count(expected: &BTreeMap<String, String>) -> usize {
    expected.keys().filter(|k| k.contains('#')).count()
}

fn delivery_orders(count: usize) -> BoxedStrategy<Vec<Vec<usize>>> {
    let size = seed_edits().len().saturating_add(count);
    prop::collection::vec(prop::collection::vec(0..size, size), size)
        .prop_map(move |keys| keys.iter().map(|k| full_shuffle(k.as_slice())).collect())
        .boxed()
}

fn tail_strategy(count: usize) -> BoxedStrategy<Vec<FieldEdit>> {
    prop::collection::vec(edit_strategy(&[0usize, 1, 2]), count).boxed()
}

#[test]
fn merge_prefers_the_later_edit() {
    let base = EPOCH.saturating_add(1_000);
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
    let base = EPOCH.saturating_add(1_000);
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
fn corrected_time_clamps_to_the_received_instant() {
    let received = TimestampMs::from_millis(EPOCH.saturating_add(500));
    let edited = TimestampMs::from_millis(EPOCH.saturating_add(1_000));
    assert_eq!(
        corrected_time(edited, 0, received),
        EPOCH.saturating_add(500)
    );
    assert_eq!(
        corrected_time(edited, -700, received),
        EPOCH.saturating_add(300)
    );
    let early = TimestampMs::from_millis(EPOCH.saturating_add(100));
    assert_eq!(corrected_time(early, -100, received), EPOCH);
    assert_eq!(
        corrected_time(early, -500_000, received),
        EPOCH.saturating_add(100).saturating_sub(500_000)
    );
    assert_eq!(
        corrected_time(TimestampMs::from_millis(100), -500_000, received),
        0
    );
    let future = TimestampMs::from_millis(u64::MAX);
    assert_eq!(
        corrected_time(future, 1, received),
        EPOCH.saturating_add(500)
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn merge_matches_independent_max_winner(tail in tail_strategy(1_000)) {
        let edits = [seed_edits(), tail].concat();
        let mut state = SettingsState::default();
        state.apply_all(&edits);
        let expected = expected_values(&edits);
        prop_assert!(!expected.is_empty());
        prop_assert!(list_key_count(&expected) >= 4, "keys {:?}", expected.keys().collect::<Vec<_>>());
        for (key, want) in &expected {
            prop_assert_eq!(state.value_of(key), Some(want.clone()), "key {}", key.clone());
        }
    }

    #[test]
    fn merge_converges_under_random_orders_and_repeats(
        parts in (tail_strategy(1_000), delivery_orders(1_000)),
    ) {
        let (tail, orders) = parts;
        let edits = [seed_edits(), tail].concat();
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
            state.apply_all(&sent);
        }
        let expected = expected_values(&edits);
        prop_assert!(!expected.is_empty());
        prop_assert!(list_key_count(&expected) >= 4);
        for state in &states {
            for (key, want) in &expected {
                prop_assert_eq!(state.value_of(key), Some(want.clone()), "key {}", key.clone());
            }
        }
        prop_assert!(agreed(&states));
    }
}
