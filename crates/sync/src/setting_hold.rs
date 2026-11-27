use anybot_protocol::{SETTING_KIND, SettingContent};
use anybot_store_client::{ACCOUNT_STREAM, OutboxRow};

use crate::core::Core;
use crate::error::SyncError;

fn is_unsent_setting(row: &OutboxRow) -> bool {
    row.kind == SETTING_KIND && row.attempted_at_ms.is_none()
}

pub(crate) fn sendable(core: &Core, rows: Vec<OutboxRow>) -> Vec<OutboxRow> {
    if core.pace().offset_ms.is_some() {
        return rows;
    }
    rows.into_iter()
        .take_while(|row| !is_unsent_setting(row))
        .collect()
}

pub(crate) fn freeze_offset(core: &Core, row: &mut OutboxRow) -> bool {
    if !is_unsent_setting(row) {
        return true;
    }
    let Some(offset) = core.pace().offset_ms else {
        return false;
    };
    let Ok(mut content) = serde_json::from_value::<SettingContent>(row.content.clone()) else {
        return true;
    };
    content.offset_ms = offset;
    if let Ok(frozen) = serde_json::to_value(&content) {
        row.content = frozen;
    }
    true
}

pub(crate) async fn any_held(core: &Core) -> Result<bool, SyncError> {
    if core.pace().offset_ms.is_some() {
        return Ok(false);
    }
    let rows = core.store.outbox_pending(ACCOUNT_STREAM, 1000).await?;
    Ok(rows.iter().any(is_unsent_setting))
}
