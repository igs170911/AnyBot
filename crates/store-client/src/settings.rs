use anybot_domain::settings_merge::EditTiming;
use anybot_domain::{DeviceId, FieldEdit, SettingsState};
use anybot_foundation::TimestampMs;
use anybot_protocol::SettingContent;

use crate::error::StoreError;
use crate::model::ACCOUNT_STREAM;
use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingValue {
    pub field: String,
    pub item: Option<String>,
    pub value: String,
}

fn edit_of(record_id: &str, content_text: &str, seq: u64) -> Result<FieldEdit, StoreError> {
    let content: SettingContent = serde_json::from_str(content_text)
        .map_err(|e| StoreError::BadValue(format!("setting content: {e}")))?;
    let device = DeviceId::parse(record_id)
        .map_err(|e| StoreError::BadValue(format!("setting record id: {e:?}")))?;
    let changed = u64::try_from(content.changed_at_ms)
        .map_err(|e| StoreError::BadValue(format!("setting time: {e}")))?;
    let timing = EditTiming {
        device_time: TimestampMs::from_millis(changed),
        clock_offset_ms: content.offset_ms,
        received_ms: TimestampMs::from_millis(i64::MAX.cast_unsigned()),
    };
    FieldEdit::new(
        content.field,
        content.item,
        device,
        timing,
        seq,
        content.value,
    )
    .map_err(|e| StoreError::BadValue(format!("setting time: {e:?}")))
}

impl Store {
    pub async fn settings(&self) -> Result<Vec<SettingValue>, StoreError> {
        let mut state = SettingsState::default();
        let stored = sqlx::query!(
            "SELECT seq, record_id, content FROM records WHERE stream = ? AND kind = 'setting'
                ORDER BY seq",
            ACCOUNT_STREAM
        )
        .fetch_all(&self.pool)
        .await?;
        for row in stored {
            let seq = u64::try_from(row.seq).map_err(|e| StoreError::BadValue(e.to_string()))?;
            state.apply(&edit_of(&row.record_id, &row.content, seq)?);
        }
        let pending = sqlx::query!(
            "SELECT record_id, content FROM outbox WHERE stream = ? AND kind = 'setting'
                AND state = 'pending' ORDER BY local_id",
            ACCOUNT_STREAM
        )
        .fetch_all(&self.pool)
        .await?;
        for row in pending {
            state.apply(&edit_of(&row.record_id, &row.content, u64::MAX)?);
        }
        Ok(state
            .winners
            .into_values()
            .map(|edit| SettingValue {
                field: edit.field,
                item: edit.list_item,
                value: edit.value,
            })
            .collect())
    }
}
