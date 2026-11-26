use axum::http::StatusCode;

use anybot_domain::{AcceptContext, AcceptOutcome};
use anybot_messages::MessageCode;
use anybot_protocol::{MAX_APPEND_ITEMS, RecordItem, StreamSelector, selector_key};
use anybot_store_server::{Pool, accounts, bots, streams};

use crate::caller::Caller;
use crate::req_write::{CallerDenied, denied, internal_sqlx};

pub struct ResolvedStream {
    pub key: String,
    pub owner: String,
    pub bot: Option<bots::BotRow>,
    pub stream_exists: bool,
}

pub async fn resolve_stream(
    pool: &Pool,
    caller: &Caller,
    selector: &StreamSelector,
) -> Result<ResolvedStream, CallerDenied> {
    let key = selector_key(selector, &caller.account);
    let stream = streams::load_stream(pool, &key)
        .await
        .map_err(internal_sqlx)?;
    match selector {
        StreamSelector::Account => {
            let stored = stream
                .as_ref()
                .ok_or(denied(StatusCode::FORBIDDEN, MessageCode::StreamBotMissing))?;
            Ok(ResolvedStream {
                key,
                owner: stored.owner.clone(),
                bot: None,
                stream_exists: true,
            })
        }
        StreamSelector::Bot { id } => {
            let bot = bots::load_bot(pool, id).await.map_err(internal_sqlx)?;
            let Some(bot) = bot else {
                return Err(denied(StatusCode::FORBIDDEN, MessageCode::StreamBotMissing));
            };
            Ok(ResolvedStream {
                key,
                owner: bot.account.clone(),
                bot: Some(bot),
                stream_exists: stream.is_some(),
            })
        }
    }
}

impl ResolvedStream {
    pub async fn account_disabled(&self, pool: &Pool) -> Result<bool, CallerDenied> {
        let row = accounts::find_account(pool, &self.owner)
            .await
            .map_err(internal_sqlx)?;
        let stored = row.ok_or(crate::req_write::internal_detail("account row missing"))?;
        Ok(stored.disabled)
    }

    #[must_use]
    pub fn accept_context(&self, caller: &Caller, disabled: bool) -> AcceptContext {
        let owner = caller.account == self.owner;
        let bot_state = self.bot.as_ref().map(|b| {
            if b.deleted {
                anybot_domain::BotState::Deleted
            } else {
                anybot_domain::BotState::Live
            }
        });
        AcceptContext {
            owner,
            account_disabled: disabled,
            stream_exists: self.stream_exists,
            bot_state,
        }
    }
}

pub fn denial_code(outcome: AcceptOutcome) -> Result<MessageCode, String> {
    let code = match outcome {
        AcceptOutcome::Accept => return Err("accepted items carry no denial code".to_owned()),
        AcceptOutcome::NoPermission => MessageCode::RecordNoPermission,
        AcceptOutcome::AccountDisabled => MessageCode::AccountLoginDisabled,
        AcceptOutcome::ForbiddenKind => MessageCode::AppendKindForbidden,
        AcceptOutcome::StreamMissing | AcceptOutcome::BotNotCreated => {
            MessageCode::StreamBotMissing
        }
        AcceptOutcome::BotDeleted => MessageCode::RecordBotDeleted,
    };
    Ok(code)
}

#[must_use]
pub fn too_many_items(items: &[RecordItem]) -> bool {
    items.len() > MAX_APPEND_ITEMS
}
