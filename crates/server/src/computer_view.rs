use anybot_domain::computer_presence::{OfflineReason, Presence, PresenceInput, presence_of};
use anybot_protocol::ComputerView;
use anybot_store_server::computers::ComputerRow;

use crate::app::AppState;
use crate::req_write::{CallerDenied, internal_detail, internal_text};

pub(crate) fn unsigned(value: i64) -> Result<u64, CallerDenied> {
    u64::try_from(value).map_err(internal_text)
}

pub(crate) fn signed(value: u64) -> Result<i64, CallerDenied> {
    i64::try_from(value).map_err(internal_text)
}

pub(crate) fn presence_now(
    state: &AppState,
    row: &ComputerRow,
    now_ms: u64,
) -> Result<Presence, CallerDenied> {
    if !row.online {
        let at = row
            .offline_at_ms
            .ok_or_else(|| internal_detail("offline computer without a time"))?;
        return Ok(Presence::Offline {
            at_ms: unsigned(at)?,
            reason: row
                .offline_reason
                .as_deref()
                .and_then(OfflineReason::from_code),
        });
    }
    Ok(presence_of(
        now_ms,
        &PresenceInput {
            stored_last_seen_ms: unsigned(row.last_seen_ms)?,
            seen_since_start_ms: state.presence.last_request(&row.device_id),
            started_ms: state.presence.started_ms(),
        },
    ))
}

pub(crate) fn is_online(
    state: &AppState,
    row: &ComputerRow,
    now_ms: u64,
) -> Result<bool, CallerDenied> {
    Ok(presence_now(state, row, now_ms)? == Presence::Online)
}

pub(crate) fn view_of(
    state: &AppState,
    row: &ComputerRow,
    now_ms: u64,
) -> Result<ComputerView, CallerDenied> {
    let (online, offline_at_ms, offline_reason) = match presence_now(state, row, now_ms)? {
        Presence::Online => (true, None, None),
        Presence::Offline { at_ms, reason } => (
            false,
            Some(signed(at_ms)?),
            reason.map(|reason| reason.code().to_owned()),
        ),
    };
    Ok(ComputerView {
        device_id: row.device_id.clone(),
        name: row.name.clone(),
        online,
        offline_at_ms,
        offline_reason,
        delegate_enabled: row.delegate_enabled,
        paired: row.paired,
    })
}
