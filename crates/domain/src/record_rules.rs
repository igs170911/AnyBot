#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptOutcome {
    Accept,
    NoPermission,
    AccountDisabled,
    ForbiddenKind,
    StreamMissing,
    BotNotCreated,
    BotDeleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState {
    Missing,
    Live,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptContext {
    pub owner: bool,
    pub account_disabled: bool,
    pub stream_exists: bool,
    pub bot_state: Option<BotState>,
}

pub const DEVICE_FORBIDDEN_KINDS: [&str; 10] = [
    "referee_cancel",
    "referee_stop",
    "referee_wake",
    "referee_card_answer",
    "referee_done_criteria",
    "record_delete",
    "record_purge",
    "record_restore",
    "record_purge_permanent",
    "bots_purge",
];

impl AcceptContext {
    #[must_use]
    pub fn account_stream(owner: bool, account_disabled: bool) -> AcceptContext {
        AcceptContext {
            owner,
            account_disabled,
            stream_exists: true,
            bot_state: None,
        }
    }

    #[must_use]
    pub fn bot_stream(
        owner: bool,
        account_disabled: bool,
        stream_exists: bool,
        bot_state: BotState,
    ) -> AcceptContext {
        AcceptContext {
            owner,
            account_disabled,
            stream_exists,
            bot_state: Some(bot_state),
        }
    }
}

#[must_use]
pub fn decide_accept(kind: &str, ctx: &AcceptContext) -> AcceptOutcome {
    if !ctx.owner {
        return AcceptOutcome::NoPermission;
    }
    if ctx.account_disabled {
        return AcceptOutcome::AccountDisabled;
    }
    if is_device_forbidden(kind) {
        return AcceptOutcome::ForbiddenKind;
    }
    if !ctx.stream_exists {
        return AcceptOutcome::StreamMissing;
    }
    match ctx.bot_state {
        Some(BotState::Missing) => return AcceptOutcome::BotNotCreated,
        Some(BotState::Deleted) => return AcceptOutcome::BotDeleted,
        Some(BotState::Live) | None => {}
    }
    AcceptOutcome::Accept
}

#[must_use]
pub fn is_device_forbidden(kind: &str) -> bool {
    DEVICE_FORBIDDEN_KINDS.contains(&kind)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupFields {
    pub kind: String,
    pub content: serde_json::Value,
    pub task_id: Option<String>,
    pub created_at_ms: i64,
}

#[must_use]
pub fn dedup_same(first: &DedupFields, second: &DedupFields) -> bool {
    first.kind == second.kind
        && first.content == second.content
        && first.task_id == second.task_id
        && first.created_at_ms == second.created_at_ms
}
