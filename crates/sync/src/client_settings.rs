use anybot_protocol::{SETTING_KIND, SettingContent};
use anybot_store_client::SettingValue;

use crate::client::{NewRecord, StreamTarget, SyncClient};
use crate::error::SyncError;

impl SyncClient {
    pub async fn change_setting(
        &self,
        field: &str,
        item: Option<&str>,
        value: &str,
    ) -> Result<String, SyncError> {
        let content = SettingContent {
            field: field.to_owned(),
            item: item.map(str::to_owned),
            value: value.to_owned(),
            changed_at_ms: self.core.now_ms()?,
            offset_ms: self.core.pace().offset_ms.unwrap_or(0),
        };
        let content =
            serde_json::to_value(&content).map_err(|e| SyncError::Encode(e.to_string()))?;
        self.add_record(NewRecord {
            stream: StreamTarget::Account,
            kind: SETTING_KIND.to_owned(),
            content,
            task_id: None,
        })
        .await
    }

    pub async fn settings(&self) -> Result<Vec<SettingValue>, SyncError> {
        Ok(self.core.store.settings().await?)
    }
}
