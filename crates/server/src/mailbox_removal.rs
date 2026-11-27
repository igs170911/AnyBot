use anybot_domain::mailbox_route::ReassignReason;
use anybot_store_server::{Pool, mailbox};

use crate::app::AppState;
use crate::computer_view::signed;
use crate::mailbox_reassign::reassign;
use crate::req_write::{CallerDenied, internal_sqlx};

pub(crate) async fn settle_removed(
    state: &AppState,
    pool: &Pool,
    device_id: &str,
) -> Result<(), CallerDenied> {
    let waiting = mailbox::pending_rounds_of(pool, device_id)
        .await
        .map_err(internal_sqlx)?;
    for letter in &waiting {
        reassign(state, pool, letter, ReassignReason::Offline).await?;
    }
    let now = signed(state.clock.now_ms().millis())?;
    mailbox::retract_cards_of(pool, device_id, now)
        .await
        .map_err(internal_sqlx)?;
    Ok(())
}
