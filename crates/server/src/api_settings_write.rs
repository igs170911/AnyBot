use anybot_domain::settings_merge::EditTiming;
use anybot_domain::settings_registry::{SettingCheck, check_setting};
use anybot_domain::{FieldEdit, SettingsState, corrected_time};
use anybot_foundation::{DeviceId, TimestampMs};
use anybot_messages::MessageCode;
use anybot_protocol::{AppendResult, RecordItem, SETTING_KIND, SettingContent};
use anybot_store_server::streams_settings::{self, SettingRow};
use anybot_store_server::{Pool, streams, streams_action};

use crate::req_write::{CallerDenied, internal_sqlx, internal_text};

const BOT_NAME_FIELD: &str = "bot_name";

pub async fn accept_setting(
    pool: &Pool,
    stream_key: &str,
    item: &RecordItem,
    now: i64,
) -> Result<AppendResult, CallerDenied> {
    let content = match checked_content(item, now) {
        Ok(content) => content,
        Err(code) => return Ok(AppendResult::rejected(code.as_str().to_owned())),
    };
    let stored = serde_json::to_string(&item.content).map_err(|e| internal_text(e.to_string()))?;
    let mut tx = streams_action::begin(pool).await.map_err(internal_sqlx)?;
    let spec = streams::InsertSpec {
        device_id: &item.device_id,
        kind: SETTING_KIND,
        content: &stored,
        created_at_ms: content.changed_at_ms,
        offline_sent: item.offline_sent,
        task_id: None,
    };
    let seq = streams::insert_in_tx(&mut tx, stream_key, &spec)
        .await
        .map_err(internal_sqlx)?;
    let corrected = corrected_of(&content, now)?;
    streams_settings::set_timing_in(&mut tx, stream_key, seq, now, corrected)
        .await
        .map_err(internal_sqlx)?;
    apply_bot_names(&mut tx, stream_key).await?;
    tx.commit().await.map_err(internal_sqlx)?;
    Ok(AppendResult::accepted(seq))
}

fn checked_content(item: &RecordItem, now: i64) -> Result<SettingContent, MessageCode> {
    let content: SettingContent =
        serde_json::from_value(item.content.clone()).map_err(|_| MessageCode::SettingMalformed)?;
    match check_setting(&content.field) {
        SettingCheck::Unknown => return Err(MessageCode::SettingUnknown),
        SettingCheck::DeviceOnly => return Err(MessageCode::SettingDeviceOnly),
        SettingCheck::Synced { per_item } => {
            if per_item != content.item.is_some() {
                return Err(MessageCode::SettingMalformed);
            }
        }
    }
    corrected_of(&content, now).map_err(|_| MessageCode::SettingMalformed)?;
    Ok(content)
}

fn corrected_of(content: &SettingContent, now: i64) -> Result<i64, CallerDenied> {
    let device = TimestampMs::from_millis(
        u64::try_from(content.changed_at_ms).map_err(|e| internal_text(e.to_string()))?,
    );
    let received =
        TimestampMs::from_millis(u64::try_from(now).map_err(|e| internal_text(e.to_string()))?);
    let corrected = corrected_time(device, content.offset_ms, received)
        .map_err(|e| internal_text(format!("{e:?}")))?;
    i64::try_from(corrected).map_err(|e| internal_text(e.to_string()))
}

pub async fn apply_bot_names(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stream_key: &str,
) -> Result<(), CallerDenied> {
    let rows = streams_settings::setting_rows_in(tx, stream_key)
        .await
        .map_err(internal_sqlx)?;
    let state = settings_state(&rows)?;
    for edit in state.winners.values() {
        if edit.field != BOT_NAME_FIELD {
            continue;
        }
        if let Some(bot) = &edit.list_item {
            streams_settings::rename_bot_in(tx, bot, &edit.value)
                .await
                .map_err(internal_sqlx)?;
        }
    }
    Ok(())
}

pub fn settings_state(rows: &[SettingRow]) -> Result<SettingsState, CallerDenied> {
    let mut state = SettingsState::default();
    for row in rows {
        let edit = edit_of(row)?;
        state.apply(&edit);
    }
    Ok(state)
}

fn edit_of(row: &SettingRow) -> Result<FieldEdit, CallerDenied> {
    let content: SettingContent =
        serde_json::from_str(&row.content).map_err(|e| internal_text(e.to_string()))?;
    let device = DeviceId::parse(&row.device_id).map_err(|e| internal_text(format!("{e:?}")))?;
    let received = row
        .received_ms
        .ok_or_else(|| internal_text("setting row without received time"))?;
    let timing = EditTiming {
        device_time: TimestampMs::from_millis(
            u64::try_from(content.changed_at_ms).map_err(|e| internal_text(e.to_string()))?,
        ),
        clock_offset_ms: content.offset_ms,
        received_ms: TimestampMs::from_millis(
            u64::try_from(received).map_err(|e| internal_text(e.to_string()))?,
        ),
    };
    let stream_seq = u64::try_from(row.seq).map_err(|e| internal_text(e.to_string()))?;
    FieldEdit::new(
        content.field,
        content.item,
        device,
        timing,
        stream_seq,
        content.value,
    )
    .map_err(|e| internal_text(format!("{e:?}")))
}
