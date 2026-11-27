use anybot_domain::mailbox_letter::CLAIM_WINDOW_MS;
use anybot_domain::mailbox_route::ReassignReason;
use anybot_store_server::mailbox;

use crate::app::AppState;
use crate::caller_session::pool_of_state;
use crate::computer_view::signed;
use crate::mailbox_reassign::{bot_computer_id, reassign};
use crate::req_write::{CallerDenied, internal_text};

pub(crate) async fn sweep_letters(state: &AppState) -> Result<(), CallerDenied> {
    let pool = pool_of_state(state)?;
    let runner = bot_computer_id(&pool).await?;
    let now = signed(state.clock.now_ms().millis())?;
    let window = i64::try_from(CLAIM_WINDOW_MS).map_err(internal_text)?;
    let floor = now.saturating_sub(window);
    let unclaimed = mailbox::expired_rounds(&pool, &runner, floor)
        .await
        .map_err(crate::req_write::internal_sqlx)?;
    for letter in &unclaimed {
        reassign(state, &pool, letter, ReassignReason::Offline).await?;
    }
    let stranded = mailbox::rounds_for_gone_computers(&pool)
        .await
        .map_err(crate::req_write::internal_sqlx)?;
    for letter in stranded.iter().filter(|letter| letter.device_id != runner) {
        reassign(state, &pool, letter, ReassignReason::Offline).await?;
    }
    Ok(())
}
