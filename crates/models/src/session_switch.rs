use crate::handoff::{self, SwitchReason};
use crate::session::Session;
use crate::types::ProviderEndpoint;

pub(crate) enum ChainStep {
    Next(crate::GatewayFail),
    Stop(crate::GatewayFail),
}

pub(crate) fn keep_or_stop(
    session: &mut Session,
    model: &crate::session::ModelRef,
    err: crate::GatewayFail,
) -> ChainStep {
    let crate::GatewayFail::Classified {
        classified,
        notices,
    } = err
    else {
        return ChainStep::Stop(err);
    };
    session.notices.extend(notices.clone());
    session.logs.push(crate::failure_log::failure_log(
        &classified,
        u32::try_from(session.logs.len().saturating_add(1)).unwrap_or(u32::MAX),
        notice_detail(&notices),
        None,
    ));
    if classified.code == "model-budget" {
        return ChainStep::Stop(crate::ask::classified_fail(
            classified,
            std::mem::take(&mut session.notices),
        ));
    }
    if !matches!(classified.fallback, crate::errors::FallbackPlan::Switch) {
        remember(session, &model.endpoint, &classified);
        return ChainStep::Stop(crate::ask::classified_fail(
            classified,
            std::mem::take(&mut session.notices),
        ));
    }
    let now = session.clock_now();
    crate::session_rest::rest(&mut session.resting, &model.endpoint, now);
    mark_failed(
        session,
        model,
        crate::session_step::switch_reason(&classified),
        classified.code,
    );
    ChainStep::Next(crate::ask::classified_fail(
        classified,
        session.notices.clone(),
    ))
}

pub(crate) fn finish_ask(
    session: &mut Session,
    result: crate::types::TurnResult,
    skips: Vec<crate::notice::Notice>,
) -> crate::session::AskOutcome {
    let notices = finish_notices(session, result.stop);
    let handoffs = std::mem::take(&mut session.handoffs);
    crate::session::AskOutcome {
        result,
        notices,
        handoffs,
        skips,
    }
}

fn notice_detail(notices: &[crate::notice::Notice]) -> &str {
    notices
        .iter()
        .find_map(|notice| {
            notice
                .params
                .iter()
                .find(|(key, _)| key == "detail")
                .map(|(_, value)| value.as_str())
        })
        .unwrap_or("")
}

fn finish_notices(
    session: &mut Session,
    stop: crate::types::StopKind,
) -> Vec<crate::notice::Notice> {
    let mut notices = std::mem::take(&mut session.notices);
    if let Some(code) = crate::notice::stop_note(stop) {
        notices.push(crate::notice::Notice {
            code,
            params: Vec::new(),
        });
    }
    notices
}

pub(crate) fn note_active(session: &mut Session, model: &crate::session::ModelRef) {
    record_failed(session, model);
    consume_pending(session, model);
    session.active = Some(crate::model_copy::copy_model(model));
}

pub(crate) fn mark_failed(
    session: &mut Session,
    model: &crate::session::ModelRef,
    reason: SwitchReason,
    kind_code: &str,
) {
    session.failed = Some(crate::model_copy::copy_model(model));
    session.failed_reason = Some(reason);
    session.failed_code = Some(kind_code.to_owned());
}

pub(crate) fn remember(
    session: &mut Session,
    endpoint: &ProviderEndpoint,
    classified: &crate::errors::Classified,
) {
    if let Some(limit) = classified.limit {
        crate::session_rest::note_too_long(session.learned_rows(), endpoint, limit);
    }
}

fn record_failed(session: &mut Session, sent: &crate::session::ModelRef) {
    let Some(before) = session.failed.take() else {
        return;
    };
    let reason = match session.failed_reason.take() {
        Some(reason) => reason,
        None => SwitchReason::Fallback,
    };
    if same_model(&before, sent) {
        session.failed_code.take();
        return;
    }
    let kind_code = session.failed_code.take();
    push_handoff(session, &before, sent, reason, kind_code.as_deref());
}

fn consume_pending(session: &mut Session, sent: &crate::session::ModelRef) {
    let Some((_wanted, reason)) = crate::session_io::take_matching(session, sent) else {
        note_return(session, sent);
        return;
    };
    if let Some(before) = session.active.as_ref()
        && !same_model(before, sent)
    {
        let before = crate::model_copy::copy_model(before);
        push_handoff(session, &before, sent, reason, None);
    }
    session.adopted = Some(crate::model_copy::copy_model(sent));
}

fn note_return(session: &mut Session, sent: &crate::session::ModelRef) {
    let Some(before) = session.active.as_ref() else {
        return;
    };
    if same_model(before, sent) {
        return;
    }
    let Some(adopted) = session.adopted.as_ref() else {
        return;
    };
    if !same_model(adopted, before) || same_model(adopted, sent) {
        return;
    }
    let before = crate::model_copy::copy_model(before);
    push_handoff(session, &before, sent, SwitchReason::User, None);
}

fn push_handoff(
    session: &mut Session,
    before: &crate::session::ModelRef,
    sent: &crate::session::ModelRef,
    reason: SwitchReason,
    kind_code: Option<&str>,
) {
    let (code, params) = handoff::reason_parts(reason, kind_code);
    session.handoffs.push(handoff::record(
        before.endpoint.kind,
        &before.endpoint.model,
        sent.endpoint.kind,
        &sent.endpoint.model,
        reason,
        code,
        params,
    ));
}

fn same_model(left: &crate::session::ModelRef, right: &crate::session::ModelRef) -> bool {
    left.endpoint.kind == right.endpoint.kind
        && left.endpoint.model == right.endpoint.model
        && left.endpoint.base_url == right.endpoint.base_url
}
