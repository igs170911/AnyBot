use std::collections::BTreeMap;

use anybot_foundation::{DeviceId, TimestampMs};

pub type FieldKey = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditTiming {
    pub device_time: TimestampMs,
    pub clock_offset_ms: i64,
    pub received_ms: TimestampMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectedTimeError {
    Overflow,
    BeforeEpoch,
}

#[must_use = "the corrected time is the clock the merge compares edits on"]
pub fn corrected_time(
    device_time: TimestampMs,
    offset_ms: i64,
    received_ms: TimestampMs,
) -> Result<u64, CorrectedTimeError> {
    let Some(device) = i64::try_from(device_time.millis()).ok() else {
        return Err(CorrectedTimeError::Overflow);
    };
    let Some(received) = i64::try_from(received_ms.millis()).ok() else {
        return Err(CorrectedTimeError::Overflow);
    };
    let Some(sum) = device.checked_add(offset_ms) else {
        return Err(CorrectedTimeError::Overflow);
    };
    if sum < 0 {
        return Err(CorrectedTimeError::BeforeEpoch);
    }
    Ok(sum.min(received).cast_unsigned())
}

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
    corrected_ms: u64,
}

impl FieldEdit {
    pub fn new(
        field: FieldKey,
        list_item: Option<String>,
        device: DeviceId,
        timing: EditTiming,
        stream_seq: u64,
        value: String,
    ) -> Result<Self, CorrectedTimeError> {
        let corrected_ms = corrected_time(
            timing.device_time,
            timing.clock_offset_ms,
            timing.received_ms,
        )?;
        Ok(Self {
            field,
            list_item,
            device,
            device_time: timing.device_time,
            clock_offset_ms: timing.clock_offset_ms,
            received_ms: timing.received_ms,
            stream_seq,
            value,
            corrected_ms,
        })
    }

    #[must_use = "the corrected time stored at construction is what the merge compares"]
    pub fn corrected_ms(&self) -> u64 {
        self.corrected_ms
    }

    #[must_use = "the key is the slot a winner competes for"]
    pub fn key(&self) -> FieldKey {
        match &self.list_item {
            Some(item) => format!("{}#{item}", self.field),
            None => self.field.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub winners: BTreeMap<FieldKey, FieldEdit>,
}

impl SettingsState {
    #[must_use = "the verdict decides whether the incoming edit beats the stored winner"]
    pub fn wins(incoming: &FieldEdit, existing: &FieldEdit) -> bool {
        let later = incoming.corrected_ms > existing.corrected_ms;
        let same_time = incoming.corrected_ms == existing.corrected_ms;
        let bigger_device = same_time && incoming.device.as_str() > existing.device.as_str();
        let same_device = same_time && incoming.device.as_str() == existing.device.as_str();
        let bigger_seq = same_device && incoming.stream_seq > existing.stream_seq;
        later || bigger_device || bigger_seq
    }

    #[must_use = "the verdict decides whether the stored winner stays"]
    pub fn keeps_existing(incoming: &FieldEdit, existing: &FieldEdit) -> bool {
        Self::wins(existing, incoming)
    }

    pub fn apply(&mut self, edit: &FieldEdit) {
        let key = edit.key();
        match self.winners.get(&key) {
            Some(existing) if Self::keeps_existing(edit, existing) => {}
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
