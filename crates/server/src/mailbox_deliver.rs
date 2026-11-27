use axum::http::StatusCode;
use serde_json::json;

use anybot_domain::mailbox_letter::LetterKind;
use anybot_domain::mailbox_route::{
    Availability, Origin, Route, RouteInput, Target, choose_route, reassign_reason,
};
use anybot_messages::MessageCode;
use anybot_protocol::{RoundReply, RoundRequest};
use anybot_store_server::mailbox::{self, NewLetter};
use anybot_store_server::{Pool, bots, computers};

use crate::app::AppState;
use crate::caller::{Caller, CredentialKind};
use crate::computer_view::{is_online, signed};
use crate::computers_service::find_owned;
use crate::mailbox_reassign::{Settled, bot_computer_id, write_line};
use crate::req_write::{CallerDenied, denied, internal_sqlx};

fn origin_of(caller: &Caller) -> Origin<'_> {
    if caller.credential == CredentialKind::Computer {
        Origin::Computer(&caller.device_id)
    } else {
        Origin::Phone
    }
}

async fn live_bot(pool: &Pool, caller: &Caller, id: &str) -> Result<bots::BotRow, CallerDenied> {
    let bot = bots::load_bot(pool, id).await.map_err(internal_sqlx)?;
    bot.filter(|bot| bot.account == caller.account && !bot.deleted)
        .ok_or(denied(StatusCode::FORBIDDEN, MessageCode::StreamBotMissing))
}

async fn availability_of(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    device_id: &str,
) -> Result<Option<Availability>, CallerDenied> {
    let row = computers::find_by_device(pool, device_id)
        .await
        .map_err(internal_sqlx)?;
    let Some(row) = row.filter(|row| row.account == caller.account && row.removed_at_ms.is_none())
    else {
        return Ok(None);
    };
    let now = state.clock.now_ms().millis();
    Ok(Some(Availability {
        online: is_online(state, &row, now)?,
        delegate_enabled: row.delegate_enabled,
    }))
}

async fn named_of(
    pool: &Pool,
    caller: &Caller,
    req: &RoundRequest,
) -> Result<Option<String>, CallerDenied> {
    match &req.computer {
        Some(id) => Ok(Some(find_owned(pool, &caller.account, id).await?.device_id)),
        None => Ok(None),
    }
}

pub(crate) async fn deliver(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    req: &RoundRequest,
) -> Result<RoundReply, CallerDenied> {
    let bot = live_bot(pool, caller, &req.bot).await?;
    let named = named_of(pool, caller, req).await?;
    let origin = origin_of(caller);
    let input = RouteInput {
        origin,
        team_member: false,
        model_only_on: None,
        named: named.as_deref(),
        bot_computer: None,
    };
    let route = choose_route(&input);
    let runner = bot_computer_id(pool).await?;
    let (target, reason) = settle(state, pool, caller, &route, &origin, &runner).await?;
    let letter_id = uuid::Uuid::now_v7().to_string();
    let origin_device = match origin {
        Origin::Computer(device) => Some(device.to_owned()),
        Origin::Phone => None,
    };
    let content = json!({"bot": bot.id, "origin": origin_device}).to_string();
    let now = signed(state.clock.now_ms().millis())?;
    let fresh = NewLetter {
        letter_id: &letter_id,
        account: &caller.account,
        kind: LetterKind::Round.code(),
        device_id: &target,
        content: &content,
        now_ms: now,
    };
    mailbox::insert(pool, &fresh).await.map_err(internal_sqlx)?;
    if let Some((reason, from)) = &reason {
        let settled = Settled {
            account: &caller.account,
            bot_id: &bot.id,
            from_device: from,
            new_letter: &letter_id,
            reason: *reason,
        };
        write_line(state, pool, &settled).await?;
    }
    crate::device_notes::notify_mail(state, &target);
    Ok(RoundReply {
        letter_id,
        device_id: target,
        reassigned: reason.map(|(reason, _from)| reason.code().to_owned()),
    })
}

async fn settle(
    state: &AppState,
    pool: &Pool,
    caller: &Caller,
    route: &Route<'_>,
    origin: &Origin<'_>,
    runner: &str,
) -> Result<
    (
        String,
        Option<(anybot_domain::mailbox_route::ReassignReason, String)>,
    ),
    CallerDenied,
> {
    let Target::Device(device) = route.target else {
        return Ok((runner.to_owned(), None));
    };
    let availability = availability_of(state, pool, caller, device).await?;
    match reassign_reason(route, origin, availability) {
        Some(reason) => Ok((runner.to_owned(), Some((reason, device.to_owned())))),
        None => Ok((device.to_owned(), None)),
    }
}
