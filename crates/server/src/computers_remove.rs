use anybot_store_server::Pool;

use crate::app::AppState;
use crate::req_write::CallerDenied;

pub(crate) async fn finish_removal(
    state: &AppState,
    pool: &Pool,
    account: &str,
    device_id: &str,
) -> Result<(), CallerDenied> {
    state.presence.forget(device_id);
    crate::device_notes::revoke_device(state, device_id);
    crate::mailbox_removal::settle_removed(state, pool, device_id).await?;
    crate::computers_log::announce(state, pool, account, "removed", device_id).await
}
