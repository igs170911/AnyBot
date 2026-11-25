use crate::handoff::SwitchReason;
use crate::session::{ModelRef, Session};

pub(crate) fn peek_pending(session: &Session) -> Option<(ModelRef, SwitchReason)> {
    match session.pending.lock() {
        Ok(guard) => guard
            .as_ref()
            .map(|(model, reason)| (crate::model_copy::copy_model(model), *reason)),
        Err(poisoned) => poisoned
            .into_inner()
            .as_ref()
            .map(|(model, reason)| (crate::model_copy::copy_model(model), *reason)),
    }
}

pub(crate) fn clear_pending(session: &Session) -> Result<(), ()> {
    match session.pending.lock() {
        Ok(mut slot) => {
            *slot = None;
            Ok(())
        }
        Err(poisoned) => {
            *poisoned.into_inner() = None;
            Err(())
        }
    }
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
