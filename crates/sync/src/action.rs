use std::pin::Pin;
use std::time::Duration;

use serde_json::Value;

use crate::client::SyncClient;
use crate::core::Core;
use crate::error::SyncError;
use crate::idem::action_key;
use crate::reply_gate::{Gate, gate};
use crate::status::ConnectionState;
use crate::wire::{WireLimits, WireMethod, WireReply, WireRequest};

pub const ACTION_UNREACHABLE_CODE: &str = "account-action-unreachable";
pub const ACTION_TOTAL_SECS: u64 = 100;
pub const ACTION_GAPS_MS: [u64; 6] = [500, 1_000, 2_000, 4_000, 8_000, 8_000];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionReply {
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("{ACTION_UNREACHABLE_CODE}")]
    Unreachable,
    #[error("the session is no longer valid")]
    NeedLogin,
    #[error("this install is too old")]
    VersionTooOld,
    #[error(transparent)]
    Local(#[from] SyncError),
}

impl ActionError {
    #[must_use]
    pub fn code(&self) -> Option<&'static str> {
        match self {
            ActionError::Unreachable => Some(ACTION_UNREACHABLE_CODE),
            ActionError::NeedLogin | ActionError::VersionTooOld | ActionError::Local(_) => None,
        }
    }
}

enum Verdict {
    Done(ActionReply),
    Stop(ActionError),
    Again,
}

fn verdict_of(core: &Core, reply: &WireReply) -> Verdict {
    let done = || ActionReply {
        status: reply.status,
        body: reply.body.clone(),
    };
    match gate(core, reply) {
        Gate::Fine | Gate::Refused(_) => Verdict::Done(done()),
        Gate::Halted => Verdict::Stop(halted_error(core)),
        Gate::Trouble => Verdict::Again,
    }
}

fn halted_error(core: &Core) -> ActionError {
    if core.status.get().connection == ConnectionState::VersionTooOld {
        ActionError::VersionTooOld
    } else {
        ActionError::NeedLogin
    }
}

fn gap_after(attempt: usize) -> Duration {
    let last = ACTION_GAPS_MS.len().saturating_sub(1);
    let millis = ACTION_GAPS_MS
        .get(attempt.min(last))
        .copied()
        .unwrap_or(8_000);
    Duration::from_millis(millis)
}

struct RetryNotice<'a> {
    core: &'a Core,
    shown: bool,
}

impl RetryNotice<'_> {
    fn show(&mut self) {
        if !self.shown {
            self.shown = true;
            self.core.status.update(|status| {
                status.actions_retrying = status.actions_retrying.saturating_add(1);
            });
        }
    }
}

impl Drop for RetryNotice<'_> {
    fn drop(&mut self) {
        if self.shown {
            self.core.status.update(|status| {
                status.actions_retrying = status.actions_retrying.saturating_sub(1);
            });
        }
    }
}

type Deadline = Pin<Box<dyn Future<Output = ()> + Send>>;

async fn attempt_once(
    core: &Core,
    request: WireRequest,
    deadline: &mut Deadline,
) -> Result<Verdict, ActionError> {
    let answered = tokio::select! {
        biased;
        reply = core.call(request) => reply,
        () = deadline => return Err(ActionError::Unreachable),
    };
    let Ok(reply) = answered else {
        return Ok(Verdict::Again);
    };
    core.note_reply(&reply)?;
    Ok(verdict_of(core, &reply))
}

pub(crate) async fn run(
    core: &Core,
    action_id: &str,
    path: &str,
    body: &Value,
) -> Result<ActionReply, ActionError> {
    let text = serde_json::to_string(body).map_err(|e| SyncError::Encode(e.to_string()))?;
    let mut deadline: Deadline = Box::pin(core.clock.sleep(Duration::from_secs(ACTION_TOTAL_SECS)));
    let mut notice = RetryNotice { core, shown: false };
    let mut attempt: usize = 0;
    loop {
        let request = WireRequest {
            method: WireMethod::Post,
            path: path.to_owned(),
            body: Some(text.clone()),
            idempotency_key: Some(action_key(action_id)),
            limits: WireLimits::ACTION,
        };
        match attempt_once(core, request, &mut deadline).await? {
            Verdict::Done(done) => return Ok(done),
            Verdict::Stop(stop) => return Err(stop),
            Verdict::Again => notice.show(),
        }
        let pause = core.clock.sleep(gap_after(attempt));
        tokio::select! {
            biased;
            () = pause => {}
            () = &mut deadline => return Err(ActionError::Unreachable),
        }
        attempt = attempt.saturating_add(1);
    }
}

impl SyncClient {
    pub async fn send_action(
        &self,
        action_id: &str,
        path: &str,
        body: &Value,
    ) -> Result<ActionReply, ActionError> {
        run(&self.core, action_id, path, body).await
    }
}
