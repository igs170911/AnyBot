use std::sync::{Arc, Mutex};

use crate::handoff::SwitchReason;
use crate::session::{ModelRef, Session};

pub(crate) fn peek_pending(session: &Session) -> Option<ModelRef> {
    lock_model(&session.pending)
}

pub(crate) fn pending_reason(session: &Session) -> Option<SwitchReason> {
    session.pending_reason.lock().ok().and_then(|slot| *slot)
}

pub(crate) fn exhausted(
    last: Option<crate::GatewayFail>,
    skips: Vec<crate::notice::Notice>,
) -> crate::GatewayFail {
    match last {
        Some(err) => attach_skips(err, skips),
        None => crate::ask::classified_fail(
            crate::errors::classify(&crate::errors::FailInput::MissingKey),
            skips,
        ),
    }
}

fn attach_skips(err: crate::GatewayFail, skips: Vec<crate::notice::Notice>) -> crate::GatewayFail {
    let crate::GatewayFail::Classified {
        classified,
        mut notices,
    } = err
    else {
        return err;
    };
    notices.extend(skips);
    crate::GatewayFail::Classified {
        classified,
        notices,
    }
}

fn lock_model(slot: &Arc<Mutex<Option<ModelRef>>>) -> Option<ModelRef> {
    slot.lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(crate::model_copy::copy_model))
}
