use serde_json::Value;

use crate::client::{Gateway, GatewayFail};
use crate::types::{ProviderEndpoint, TurnResult};

pub(crate) fn recorded_now(
    gateway: &Gateway,
    endpoint: &ProviderEndpoint,
    url: &str,
    body: &Value,
    stream: bool,
) -> Option<Result<TurnResult, GatewayFail>> {
    let posted = crate::transport::Posted {
        url: url.to_owned(),
        body: body.to_string(),
    };
    gateway
        .post_script(&posted)
        .map(|outcome| recorded_outcome(endpoint, outcome, stream))
}

fn recorded_outcome(
    endpoint: &ProviderEndpoint,
    outcome: crate::transport::PostedOutcome,
    stream: bool,
) -> Result<TurnResult, GatewayFail> {
    match outcome {
        crate::transport::PostedOutcome::Fail(fail) => Err(GatewayFail::Transport(fail)),
        crate::transport::PostedOutcome::Http(bytes) => http_outcome(endpoint, &bytes, stream),
    }
}

#[cfg(test)]
fn http_outcome(
    endpoint: &ProviderEndpoint,
    bytes: &crate::transport::HttpBytes,
    stream: bool,
) -> Result<TurnResult, GatewayFail> {
    if bytes.status >= 300 && bytes.status < 400 {
        return Err(crate::client::status_fail(endpoint, bytes));
    }
    if bytes.status >= 400 {
        return Err(crate::client::status_fail(endpoint, bytes));
    }
    let text = String::from_utf8_lossy(&bytes.body);
    if stream {
        return crate::stream_fail::finish_stream(endpoint, &text, false);
    }
    Ok(crate::stream_fail::decode_body(endpoint, &text))
}
