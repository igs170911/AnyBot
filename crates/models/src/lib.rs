pub mod client;
pub(crate) mod jsonutil;
pub mod providers;
pub(crate) mod thinking;
pub(crate) mod transport;
pub mod types;
pub mod where_run;
pub(crate) mod wire_claude;
pub(crate) mod wire_claude_blocks;
pub(crate) mod wire_claude_finish;
pub(crate) mod wire_claude_usage;
pub(crate) mod wire_openai;
pub(crate) mod wire_openai_calls;
pub(crate) mod wire_openai_usage;

pub use client::{Gateway, GatewayFail, IdentityResolver, NameResolver};
pub use providers::{CLAUDE_MAX_OUTPUT, PROVIDERS, resolve_endpoint};
pub use transport::TransportFail;
pub use types::{
    Audience, Message, NativeAttachment, ProviderEndpoint, ProviderKind, Role, StopKind,
    ThinkingMode, ToolArguments, ToolCall, ToolSpec, TurnRequest, TurnResult, Usage, UsageReport,
    WireFormat,
};
pub use where_run::{CONNECT_TIMEOUT, RunPlace, STALL_AFTER_TEXT, classify_url, first_token_wait};

use anybot_foundation::clock::Clock;
use std::sync::Arc;

#[cfg(test)]
use test_harness as _;

#[must_use]
pub fn gateway(clock: Arc<dyn Clock>) -> Gateway {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    Gateway::new(client, clock)
}
