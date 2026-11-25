use std::collections::BTreeMap;

use anybot_foundation::{DeviceId, TimestampMs};

pub type FieldKey = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEdit {
    pub field: FieldKey,
    pub list_item: Option<String>,
    pub device: DeviceId,
    pub device_time: TimestampMs,
    pub clock_offset_ms: u64,
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
        self.device_time
            .millis()
            .saturating_sub(self.clock_offset_ms)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub winners: BTreeMap<FieldKey, FieldEdit>,
}

impl SettingsState {
    #[must_use]
    pub fn wins(existing: &FieldEdit, incoming: &FieldEdit) -> bool {
        let left = existing.corrected_time();
        let right = incoming.corrected_time();
        if right != left {
            return right > left;
        }
        incoming.device.as_str() > existing.device.as_str()
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

    #[must_use]
    pub fn value_of(&self, key: &FieldKey) -> Option<String> {
        self.winners.get(key).map(|e| e.value.clone())
    }
}

#[must_use]
pub fn agreed(states: &[SettingsState]) -> bool {
    let Some(first) = states.first() else {
        return true;
    };
    states.iter().all(|s| s.winners == first.winners)
}
