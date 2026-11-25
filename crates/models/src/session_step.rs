use crate::GatewayFail;
use crate::ask;
use crate::errors::{Classified, FailInput, FailKind, classify};
use crate::estimate::{EstimateInput, apply_calibration, formula_tokens};
use crate::handoff::SwitchReason;
use crate::types::{ProviderEndpoint, ProviderKind, ThinkingMode, TurnRequest, TurnResult};

pub(crate) enum Step {
    Done(TurnResult),
    DropParam,
    BlankAgain,
    Wait(ask::AttemptFail),
    Stop(GatewayFail),
}

pub(crate) fn decide(
    outcome: Result<TurnResult, GatewayFail>,
    _sent: u32,
    blanks: u32,
    dropped: bool,
) -> Step {
    match outcome {
        Ok(result) if ask::blank_result(&result) && blanks == 0 => Step::BlankAgain,
        Ok(result) => Step::Done(result),
        Err(GatewayFail::Classified { classified, .. })
            if matches!(classified.kind, FailKind::RejectedParam) && !dropped =>
        {
            Step::DropParam
        }
        Err(GatewayFail::Classified {
            classified,
            notices,
        }) => Step::Wait(ask::AttemptFail {
            retry_after: classified.retry_after,
            classified,
            detail: String::new(),
            notices,
        }),
        Err(GatewayFail::Transport(fail)) => {
            Step::Wait(ask::from_transport(&fail, ProviderKind::Custom, false))
        }
        Err(other) => Step::Stop(other),
    }
}

pub(crate) fn switch_reason(classified: &Classified) -> SwitchReason {
    match classified.kind {
        FailKind::Unreachable => SwitchReason::Unreachable,
        FailKind::MissingKey
        | FailKind::PolicyBlocked
        | FailKind::Auth
        | FailKind::Quota
        | FailKind::Busy
        | FailKind::Server
        | FailKind::FirstToken
        | FailKind::Stalled
        | FailKind::StreamCut
        | FailKind::TooLong
        | FailKind::NoTools
        | FailKind::RejectedParam
        | FailKind::NotFound
        | FailKind::StreamError
        | FailKind::Other4xx
        | FailKind::Redirect
        | FailKind::Budget => SwitchReason::Fallback,
    }
}

pub(crate) fn no_tools_fail() -> GatewayFail {
    ask::classified_fail(
        classify(&FailInput::Http {
            status: 400,
            message: "does not support tools".to_owned(),
        }),
        Vec::new(),
    )
}

pub(crate) fn budget_fail() -> GatewayFail {
    ask::classified_fail(classify(&FailInput::Budget), Vec::new())
}

pub(crate) fn request_for_model(
    model: &crate::session::ModelRef,
    primary: &crate::session::ModelRef,
    request: &TurnRequest,
    request_for: &mut Option<&mut (dyn FnMut(&ProviderEndpoint) -> TurnRequest + Send)>,
) -> TurnRequest {
    let mut built = match request_for {
        Some(callback) => callback(&model.endpoint),
        None => request.clone(),
    };
    if !same_endpoint(&model.endpoint, &primary.endpoint) {
        built.thinking = ThinkingMode::Auto;
    }
    built
}

fn same_endpoint(left: &ProviderEndpoint, right: &ProviderEndpoint) -> bool {
    left.kind == right.kind && left.model == right.model && left.base_url == right.base_url
}

pub(crate) fn over_budget(
    request: &TurnRequest,
    usable: u64,
    calibration: &crate::estimate::Calibration,
) -> bool {
    estimate_request(request, calibration) > usable
}

pub(crate) fn estimate_request(
    request: &TurnRequest,
    calibration: &crate::estimate::Calibration,
) -> u64 {
    let text = request
        .messages
        .iter()
        .map(|message| message.text.as_str())
        .fold(String::new(), |mut acc, item| {
            acc.push_str(item);
            acc
        });
    if let Some(override_tokens) = request.estimate_override {
        return override_tokens;
    }
    let raw = formula_tokens(&EstimateInput {
        text: &text,
        messages: u64::try_from(request.messages.len()).unwrap_or(0),
        images: 0,
    });
    apply_calibration(raw, calibration)
}
