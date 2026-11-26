use crate::GatewayFail;
use crate::capabilities;
use crate::session::{ModelRef, Session};
use crate::types::{Audience, ProviderEndpoint, TurnRequest, TurnResult};

pub(crate) async fn one(
    session: &mut Session,
    model: &ModelRef,
    request: &TurnRequest,
    stream: bool,
    audience: Audience,
    permit: &crate::egress::SendPermit,
) -> Result<TurnResult, GatewayFail> {
    let cap = capabilities::resolve(
        model.endpoint.kind,
        &model.endpoint.model,
        model.bot_context,
        learned_for(session, &model.endpoint),
    );
    if !cap.tools && !request.tools.is_empty() {
        return Err(crate::session_step::no_tools_fail());
    }
    let usable = capabilities::usable_context(
        cap.context_tokens,
        model.bot_context,
        learned_for(session, &model.endpoint).and_then(|row| row.context_tokens),
        request.max_output_tokens.unwrap_or(0),
    );
    let calibration = match learned_for(session, &model.endpoint) {
        Some(row) => row.calibration.clone(),
        None => crate::estimate::Calibration::new(),
    };
    if crate::session_step::over_budget(request, usable, &calibration) {
        return Err(crate::session_step::budget_fail());
    }
    crate::session_rest::send_tries(
        session,
        crate::session_rest::TrySend {
            model,
            request,
            stream,
            audience,
            calibration: &calibration,
            thinking: cap.thinking,
            permit,
        },
    )
    .await
}

fn learned_for<'a>(
    session: &'a Session,
    endpoint: &ProviderEndpoint,
) -> Option<&'a capabilities::Learned> {
    crate::session_rest::learned_for(session.learned(), endpoint)
}
