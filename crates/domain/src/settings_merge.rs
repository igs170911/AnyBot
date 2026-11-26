use std::collections::BTreeMap;

use anybot_foundation::{DeviceId, TimestampMs};

pub type FieldKey = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEdit {
    pub field: FieldKey,
    pub list_item: Option<String>,
    pub device: DeviceId,
    pub device_time: TimestampMs,
    pub clock_offset_ms: i64,
    pub received_ms: TimestampMs,
    pub stream_seq: u64,
    pub value: String,
}

impl FieldEdit {
    #[must_use]
    pub fn key(&self) -> FieldKey {
        match &self.list_item {
            Some(item) => format!("{}#{item}", self.field),
            None => self.field.clone(),
        }
    }

    #[must_use]
    pub fn corrected_time(&self) -> u64 {
        corrected_time(self.device_time, self.clock_offset_ms, self.received_ms)
    }
}

#[must_use = "the corrected time is the clock the merge compares edits on"]
pub fn corrected_time(device_time: TimestampMs, offset_ms: i64, received_ms: TimestampMs) -> u64 {
    let device_ms = i64::try_from(device_time.millis()).unwrap_or(i64::MAX);
    let shifted = device_ms.saturating_add(offset_ms);
    let received = received_ms.millis();
    u64::try_from(shifted).unwrap_or(received).min(received)
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub winners: BTreeMap<FieldKey, FieldEdit>,
}

impl SettingsState {
    #[must_use = "the verdict decides whether the incoming edit replaces the stored winner"]
    pub fn wins(incoming: &FieldEdit, existing: &FieldEdit) -> bool {
        let time = existing.corrected_time();
        let device = existing.device.as_str();
        let later = incoming.corrected_time() > time;
        let same_time = incoming.corrected_time() == time;
        let bigger_device = same_time && incoming.device.as_str() > device;
        let same_device = same_time && incoming.device.as_str() == device;
        let bigger_seq = same_device && incoming.stream_seq > existing.stream_seq;
        later || bigger_device || bigger_seq
    }

    pub fn apply(&mut self, edit: &FieldEdit) {
        let key = edit.key();
        match self.winners.get(&key) {
            Some(existing) if Self::wins(existing, edit) => {}
            _ => {
                self.winners.insert(key, edit.clone());
            }
        }
    }

    pub fn apply_all(&mut self, edits: &[FieldEdit]) {
        for edit in edits {
            self.apply(edit);
        }
    }

    #[must_use = "the stored winner value is what every device reads back"]
    pub fn value_of(&self, key: &FieldKey) -> Option<String> {
        self.winners.get(key).map(|e| e.value.clone())
    }
}

#[must_use = "agreement means every device converged to the same winners"]
pub fn agreed(states: &[SettingsState]) -> bool {
    let Some(first) = states.first() else {
        return true;
    };
    states.iter().all(|s| s.winners == first.winners)
}
