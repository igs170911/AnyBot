use crate::client::ModelChoice;
use crate::egress::SendPermit;
use crate::errors::{Classified, FailInput, FailKind, FallbackPlan, RetryPlan, classify};
use crate::notice::{Notice, missing_key_notice, skip_notice};
use crate::session::{AskOutcome, AskPlan, MAX_FALLBACKS, Session, needs_key};
use crate::session_rest::resting_now;
use crate::session_switch::{finish_ask, push_failure_log, remember, same_model};
use crate::{GatewayFail, handoff};

pub(crate) async fn ask_single(
    session: &mut Session,
    plan: &mut AskPlan<'_>,
) -> Result<AskOutcome, GatewayFail> {
    let mut skips = Vec::new();
    let permit = match check_primary(session, plan, &mut skips) {
        Ok(permit) => permit,
        Err(fail) => return Err(fail),
    };
    session.active = Some(plan.primary.copy_ref());
    let built = crate::session_step::request_for_model(
        plan.primary,
        plan.primary,
        plan.request,
        &mut plan.request_for,
    );
    match crate::session_ask::one(
        session,
        plan.primary,
        &built,
        plan.stream,
        plan.audience,
        &permit,
    )
    .await
    {
        Ok(result) => Ok(finish_ask(session, result, skips)),
        Err(err) => Err(on_step_fail(session, plan, err, skips)),
    }
}

fn check_primary(
    session: &mut Session,
    plan: &AskPlan<'_>,
    skips: &mut Vec<Notice>,
) -> Result<SendPermit, GatewayFail> {
    let now = session.clock_now();
    if resting_now(session.resting(), &plan.primary.endpoint, now) {
        return Err(exhausted_fail(session, plan, resting_classified(), skips));
    }
    let Some(permit) =
        SendPermit::for_endpoint(session.policy(), &plan.primary.endpoint, plan.use_kind)
    else {
        skips.push(skip_notice(plan.primary.endpoint.kind, plan.bot));
        let classified = classify(&FailInput::PolicyBlocked);
        return Err(exhausted_fail(session, plan, classified, skips));
    };
    if plan.primary.endpoint.api_key.is_none() && needs_key(plan.primary.endpoint.kind) {
        skips.push(missing_key_notice(plan.primary.endpoint.kind));
        let classified = classify(&FailInput::MissingKey);
        return Err(exhausted_fail(session, plan, classified, skips));
    }
    Ok(permit)
}

fn on_step_fail(
    session: &mut Session,
    plan: &AskPlan<'_>,
    err: GatewayFail,
    mut skips: Vec<Notice>,
) -> GatewayFail {
    let GatewayFail::Classified {
        classified,
        notices,
    } = err
    else {
        return err;
    };
    session.notices.extend(notices.clone());
    push_failure_log(session, &classified, &notices);
    remember(session, &plan.primary.endpoint, &classified);
    if classified.code == "model-budget" || !matches!(classified.fallback, FallbackPlan::Switch) {
        return crate::ask::classified_fail(classified, std::mem::take(&mut session.notices));
    }
    let now = session.clock_now();
    crate::session_rest::rest(&mut session.resting, &plan.primary.endpoint, now);
    exhausted_fail(session, plan, classified, &mut skips)
}

fn exhausted_fail(
    session: &mut Session,
    plan: &AskPlan<'_>,
    classified: Classified,
    skips: &mut Vec<Notice>,
) -> GatewayFail {
    let fallback = pick_fallback(session, plan, &classified, skips).map(Box::new);
    let mut combined = std::mem::take(&mut session.notices);
    combined.append(skips);
    GatewayFail::Exhausted {
        fallback,
        classified,
        notices: combined,
    }
}

fn pick_fallback(
    session: &Session,
    plan: &AskPlan<'_>,
    classified: &Classified,
    skips: &mut Vec<Notice>,
) -> Option<ModelChoice> {
    let now = session.clock_now();
    for candidate in plan.fallbacks.iter().take(MAX_FALLBACKS) {
        if same_model(candidate, plan.primary)
            || resting_now(session.resting(), &candidate.endpoint, now)
        {
            continue;
        }
        if SendPermit::for_endpoint(session.policy(), &candidate.endpoint, plan.use_kind).is_none()
        {
            skips.push(skip_notice(candidate.endpoint.kind, plan.bot));
            continue;
        }
        if candidate.endpoint.api_key.is_none() && needs_key(candidate.endpoint.kind) {
            skips.push(missing_key_notice(candidate.endpoint.kind));
            continue;
        }
        return Some(ModelChoice {
            model: candidate.copy_ref(),
            reason_code: reason_code_of(classified),
        });
    }
    None
}

fn reason_code_of(classified: &Classified) -> &'static str {
    let reason = crate::session_step::switch_reason(classified);
    handoff::reason_parts(reason, Some(classified.code)).0
}

fn resting_classified() -> Classified {
    Classified {
        kind: FailKind::Busy,
        retry: RetryPlan::None,
        fallback: FallbackPlan::Switch,
        code: "model-resting",
        http_status: None,
        limit: None,
        retry_after: None,
    }
}
