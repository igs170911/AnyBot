#![cfg(test)]

use std::collections::BTreeMap;
use std::hash::{BuildHasher, DefaultHasher, Hasher};

use proptest::prelude::*;

use anybot_foundation::{DeviceId, TimestampMs};

use crate::settings_merge::*;

pub const EPOCH: u64 = 1_700_000_000_000;
pub const LIST_FIELDS: usize = 18;
pub const LIST_ITEMS: usize = 3;
pub const SEED_FIELDS: usize = 22;

#[must_use]
pub fn device_id(index: usize) -> DeviceId {
    DeviceId::parse(&format!("00000000-0000-4000-8000-{index:0>12}")).unwrap()
}

#[must_use]
pub fn edit(
    device_index: usize,
    field: usize,
    offset: i64,
    shift: u64,
    received: u64,
    seq: u64,
) -> FieldEdit {
    FieldEdit::new(
        format!("f{field}"),
        list_item_of(field),
        device_id(device_index),
        EditTiming {
            device_time: TimestampMs::from_millis(EPOCH.checked_add(shift).unwrap()),
            clock_offset_ms: offset,
            received_ms: TimestampMs::from_millis(received),
        },
        seq,
        format!("v{device_index}-{shift}-{seq}"),
    )
    .unwrap()
}

#[must_use]
pub fn list_item_of(field: usize) -> Option<String> {
    if field >= LIST_FIELDS {
        Some(format!("i{}", field % LIST_ITEMS))
    } else {
        None
    }
}

#[must_use]
pub fn value_of_edit(edit: &FieldEdit) -> String {
    let shift = edit.device_time.millis().checked_sub(EPOCH).unwrap();
    let name = edit.device.as_str();
    let tail = name.rsplit('-').next().unwrap_or("0");
    let index: usize = tail.parse().unwrap_or(0);
    format!("v{index}-{shift}-{seq}", seq = edit.stream_seq)
}

#[must_use]
pub fn seed_edits() -> Vec<FieldEdit> {
    let mut out = Vec::new();
    for field in 0..SEED_FIELDS {
        for device in 0..3usize {
            out.push(edit(
                device,
                field,
                0,
                10,
                EPOCH + 20,
                u64::try_from(device).unwrap_or(0),
            ));
        }
    }
    out
}

#[must_use = "the strategy feeds the property tests"]
pub fn edit_strategy(devices: &[usize]) -> BoxedStrategy<FieldEdit> {
    (
        prop::sample::select(devices.to_vec()),
        0usize..SEED_FIELDS,
        -600_000i64..600_000,
        0u64..1_200_000,
        EPOCH..EPOCH.checked_add(1_200_000).unwrap(),
        0u64..1_000,
    )
        .prop_map(|(device_index, field, offset, shift, received, seq)| {
            edit(device_index, field, offset, shift, received, seq)
        })
        .boxed()
}

#[must_use]
pub fn model_corrected(edit: &FieldEdit) -> i64 {
    let device = i64::try_from(edit.device_time.millis()).unwrap();
    let received = i64::try_from(edit.received_ms.millis()).unwrap();
    let sum = device.checked_add(edit.clock_offset_ms).unwrap();
    sum.min(received)
}

#[must_use]
pub fn model_sort_key(edit: &FieldEdit) -> (i64, String, u64) {
    (
        model_corrected(edit),
        edit.device.as_str().clone(),
        edit.stream_seq,
    )
}

#[must_use]
pub fn model_key(edit: &FieldEdit) -> String {
    match &edit.list_item {
        Some(item) => format!("{}#{item}", edit.field),
        None => edit.field.clone(),
    }
}

#[must_use]
pub fn expected_values(edits: &[FieldEdit]) -> BTreeMap<String, String> {
    let mut best: BTreeMap<String, (i64, String, u64)> = BTreeMap::new();
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    for edit in edits {
        let key = model_sort_key(edit);
        match best.get(&model_key(edit)) {
            Some(top) if *top >= key => {}
            _ => {
                best.insert(model_key(edit), key);
                values.insert(model_key(edit), value_of_edit(edit));
            }
        }
    }
    values
}

#[must_use]
pub fn list_key_count(expected: &BTreeMap<String, String>) -> usize {
    expected.keys().filter(|k| k.contains('#')).count()
}

#[must_use]
fn seeded_hash(seed: u64) -> BuildStateHasher {
    let state = DefaultHasher::new().finish().wrapping_add(seed);
    BuildStateHasher(state)
}

struct BuildStateHasher(u64);

impl BuildHasher for BuildStateHasher {
    type Hasher = StateHasher;
    fn build_hasher(&self) -> StateHasher {
        StateHasher(self.0)
    }
}

struct StateHasher(u64);

impl Hasher for StateHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

#[must_use]
pub fn shuffled_order(size: usize, seed: u64) -> Vec<usize> {
    let mut pairs: Vec<(u64, usize)> = (0..size)
        .map(|i| (seeded_hash(seed).hash_one(i), i))
        .collect();
    pairs.sort_unstable();
    pairs.iter().map(|(_, i)| *i).collect()
}

#[must_use = "the strategy feeds the property tests"]
pub fn tail_strategy(count: usize) -> BoxedStrategy<Vec<FieldEdit>> {
    prop::collection::vec(edit_strategy(&[0usize, 1, 2]), count).boxed()
}
