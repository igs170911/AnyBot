use axum::http::StatusCode;

use anybot_domain::computer_name::clean_name;
use anybot_domain::computer_presence::REASON_REMOVED;
use anybot_messages::{MessageCode, Params};
use anybot_protocol::ComputersView;
use anybot_store_server::Pool;
use anybot_store_server::computers::{self, ComputerRow};

use crate::app::AppState;
use crate::computer_view::{signed, view_of};
use crate::computers_join::name_denied;
use crate::computers_remove::finish_removal;
use crate::req_write::{CallerDenied, denied, denied_with, internal_sqlx};

pub(crate) async fn list_view(
    state: &AppState,
    pool: &Pool,
    account: &str,
) -> Result<ComputersView, CallerDenied> {
    let now = state.clock.now_ms().millis();
    let rows = computers::list_live(pool, account)
        .await
        .map_err(internal_sqlx)?;
    let mut computers = Vec::with_capacity(rows.len());
    for row in &rows {
        computers.push(view_of(state, row, now)?);
    }
    Ok(ComputersView { computers })
}

pub(crate) async fn find_owned(
    pool: &Pool,
    account: &str,
    device_id: &str,
) -> Result<ComputerRow, CallerDenied> {
    let row = computers::find_by_device(pool, device_id)
        .await
        .map_err(internal_sqlx)?;
    row.filter(|row| row.account == account && row.removed_at_ms.is_none())
        .ok_or(denied(StatusCode::NOT_FOUND, MessageCode::ComputerNotFound))
}

pub(crate) async fn rename(
    state: &AppState,
    pool: &Pool,
    account: &str,
    device_id: &str,
    raw_name: &str,
) -> Result<(), CallerDenied> {
    let name = clean_name(raw_name).map_err(name_denied)?;
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    computers::lock_account_in(&mut tx, account)
        .await
        .map_err(internal_sqlx)?;
    let live = computers::list_live_in(&mut tx, account)
        .await
        .map_err(internal_sqlx)?;
    if !live.iter().any(|row| row.device_id == device_id) {
        return Err(denied(StatusCode::NOT_FOUND, MessageCode::ComputerNotFound));
    }
    if live
        .iter()
        .any(|row| row.name == name && row.device_id != device_id)
    {
        return Err(denied_with(
            StatusCode::CONFLICT,
            MessageCode::ComputerNameTaken,
            &Params::new().with_text("name", &name),
        ));
    }
    computers::rename_in(&mut tx, device_id, &name)
        .await
        .map_err(internal_sqlx)?;
    tx.commit().await.map_err(internal_sqlx)?;
    tracing::info!(device_id = %device_id, name = %name, "computer renamed");
    crate::computers_log::announce(state, pool, account, "renamed", device_id).await
}

pub(crate) async fn remove(
    state: &AppState,
    pool: &Pool,
    account: &str,
    device_id: &str,
) -> Result<(), CallerDenied> {
    let now = signed(state.clock.now_ms().millis())?;
    let mut tx = pool.begin().await.map_err(internal_sqlx)?;
    computers::lock_account_in(&mut tx, account)
        .await
        .map_err(internal_sqlx)?;
    let removed = computers::mark_removed_in(&mut tx, device_id, now, REASON_REMOVED)
        .await
        .map_err(internal_sqlx)?;
    tx.commit().await.map_err(internal_sqlx)?;
    if removed {
        finish_removal(state, pool, account, device_id).await?;
    }
    Ok(())
}
