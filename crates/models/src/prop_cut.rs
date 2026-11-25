use crate::prop_body::ReplyCase;

use crate::stream_fail::{decode_body, finish_stream};
use crate::types::{ProviderEndpoint, StopKind, ToolArguments, TurnResult, WireFormat};

pub(crate) fn send_cut(
    endpoint: &ProviderEndpoint,
    body: &str,
    at: usize,
    stream: bool,
) -> Result<TurnResult, crate::GatewayFail> {
    let step = crate::scripted::script_cut(200, body, at);
    let poster = crate::scripted::ScriptPoster::new(vec![step]);
    let gateway = crate::client::Gateway::from_wire(poster, std::sync::Arc::new(Quiet));
    let permit = crate::egress::SendPermit::for_endpoint(
        &crate::egress::DataPolicy::LocalOnly,
        endpoint,
        crate::egress::ModelUse::Bot,
    )
    .expect("permit");
    crate::prop_run::drive(gateway.complete(endpoint, &crate::prop_run::hi(), stream, &permit))
}

pub(crate) fn cut_body(raw: &str, cut: usize) -> (String, bool) {
    if raw.is_empty() {
        return (String::new(), true);
    }
    let width = raw.len().saturating_add(1);
    let at = cut.checked_rem(width).unwrap_or(0);
    if at >= raw.len() {
        return (raw.to_owned(), true);
    }
    let Some(piece) = raw.get(..at) else {
        return (raw.to_owned(), true);
    };
    (piece.to_owned(), false)
}

pub(crate) fn send_body(
    endpoint: &ProviderEndpoint,
    body: &str,
    stream: bool,
) -> Result<TurnResult, crate::GatewayFail> {
    let step = if stream {
        crate::scripted::script_chunks(200, &[body.to_owned()])
    } else {
        crate::scripted::script_http(200, body)
    };
    let poster = crate::scripted::ScriptPoster::new(vec![step]);
    let gateway = crate::client::Gateway::from_wire(poster, std::sync::Arc::new(Quiet));
    let permit = crate::egress::SendPermit::for_endpoint(
        &crate::egress::DataPolicy::LocalOnly,
        endpoint,
        crate::egress::ModelUse::Bot,
    )
    .expect("permit");
    crate::prop_run::drive(gateway.complete(endpoint, &crate::prop_run::hi(), stream, &permit))
}

struct Quiet;

impl anybot_foundation::clock::Clock for Quiet {
    fn now_ms(&self) -> anybot_foundation::clock::TimestampMs {
        anybot_foundation::clock::TimestampMs::from_millis(0)
    }

    fn sleep(&self, _: std::time::Duration) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        Box::pin(std::future::ready(()))
    }

    fn sleep_until(
        &self,
        _: anybot_foundation::clock::TimestampMs,
    ) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        Box::pin(std::future::ready(()))
    }
}

pub(crate) fn endpoint_for(case: &ReplyCase) -> ProviderEndpoint {
    let format = if case.format == 2 {
        WireFormat::Anthropic
    } else {
        WireFormat::OpenAiCompatible
    };
    crate::prop_run::custom("m", format)
}

pub(crate) fn assert_column(
    result: &TurnResult,
) -> Result<(), proptest::test_runner::TestCaseError> {
    if matches!(result.stop, StopKind::Interrupted | StopKind::Length) {
        proptest::prop_assert!(result.tool_calls.is_empty());
    }
    if matches!(result.stop, StopKind::NeedsTool) {
        proptest::prop_assert!(complete_tool(result));
    }
    if closed_tool(result) && !matches!(result.stop, StopKind::Refusal) {
        proptest::prop_assert!(matches!(result.stop, StopKind::NeedsTool));
    }
    Ok(())
}

fn closed_tool(result: &TurnResult) -> bool {
    !matches!(result.stop, StopKind::Interrupted | StopKind::Length) && complete_tool(result)
}

fn complete_tool(result: &TurnResult) -> bool {
    result
        .tool_calls
        .iter()
        .any(|call| !call.name.is_empty() && matches!(call.arguments, ToolArguments::Json(_)))
}

pub(crate) fn same_answer(left: &TurnResult, right: &TurnResult) -> bool {
    left.stop == right.stop
        && left.text == right.text
        && left.thinking == right.thinking
        && left.tool_calls == right.tool_calls
        && left.model == right.model
}

pub(crate) fn plain_decode(endpoint: &ProviderEndpoint, body: &str) -> TurnResult {
    decode_body(endpoint, body)
}

pub(crate) fn stream_decode(
    endpoint: &ProviderEndpoint,
    body: &str,
) -> Result<TurnResult, crate::GatewayFail> {
    finish_stream(endpoint, body, false)
}
