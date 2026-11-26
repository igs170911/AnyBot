use crate::handoff::SwitchReason;
use crate::session::{ModelRef, Session};

fn same_model(left: &ModelRef, right: &ModelRef) -> bool {
    left.endpoint.kind == right.endpoint.kind
        && left.endpoint.model == right.endpoint.model
        && left.endpoint.base_url == right.endpoint.base_url
}

pub(crate) fn read_pending(session: &Session) -> Option<(ModelRef, SwitchReason)> {
    let slot = match session.pending.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    slot.as_ref()
        .map(|(model, reason)| (crate::model_copy::copy_model(model), *reason))
}

pub(crate) fn take_matching(
    session: &Session,
    sent: &ModelRef,
) -> Option<(ModelRef, SwitchReason)> {
    let mut slot = match session.pending.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let matches = slot
        .as_ref()
        .is_some_and(|(model, _)| same_model(model, sent));
    if matches { slot.take() } else { None }
}

pub(crate) fn exhausted(
    last: Option<crate::GatewayFail>,
    skips: Vec<crate::notice::Notice>,
    resting: &[crate::session::RestMark],
) -> crate::GatewayFail {
    match last {
        Some(err) => attach_skips(err, skips),
        None if !resting.is_empty() => crate::ask::classified_fail(
            crate::errors::Classified {
                kind: crate::errors::FailKind::Busy,
                retry: crate::errors::RetryPlan::None,
                fallback: crate::errors::FallbackPlan::None,
                code: "model-resting",
                http_status: None,
                limit: None,
                retry_after: None,
            },
            skips,
        ),
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
