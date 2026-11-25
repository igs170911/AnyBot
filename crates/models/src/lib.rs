pub(crate) mod ask;
pub mod capabilities;
pub mod catalog;
pub(crate) mod catalog_fetch;
pub mod client;
pub mod egress;
pub mod errors;
pub(crate) mod errors_text;
pub mod estimate;
pub mod failure_log;
pub mod handoff;
pub(crate) mod jsonutil;
pub(crate) mod model_copy;
pub(crate) mod normalize_prop;
pub mod notice;
pub mod providers;
pub mod retry;
pub mod session;
pub(crate) mod session_ask;
pub(crate) mod session_chain;
pub(crate) mod session_io;
pub(crate) mod session_rest;
pub(crate) mod session_step;
pub(crate) mod session_switch;
pub(crate) mod stream_fail;
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

pub use capabilities::{
    Capability, Learned, TokenizerKind, builtin_rows, model_key, resolve, usable_context,
};
pub use catalog::{LIST_WAIT_SECS, empty_list_code, list_timeout_code, normalize_ids};
pub use catalog_fetch::{ListOutcome, fetch_models};
pub use client::{Gateway, GatewayFail, IdentityResolver, NameResolver};
pub use egress::{DataPolicy, JudgeCandidate, ModelUse, allowed, default_policy, pick_judge};
pub use errors::{
    Classified, FailInput, FailKind, FallbackPlan, RetryPlan, classify, clip_detail,
    context_limit_of as context_limit, redact,
};
pub use estimate::{
    Calibration, EstimateInput, apply_calibration, fits, formula_tokens, margin_milli,
};
pub use failure_log::{FailureLog, failure_log};
pub use handoff::{HandoffRecord, SwitchReason, handoff_code, record};
pub use normalize_prop::{Fixture, closed_tool_matches, cut_has_no_tools, normalize_pair};
pub use notice::{
    Notice, busy_notice, failure_notice, missing_key_notice, retry_notice, skip_notice, stop_note,
};
pub use providers::{CLAUDE_MAX_OUTPUT, PROVIDERS, resolve_endpoint};
pub use retry::{BUSY_WAIT, base_wait, retry_spans, try_limit};
pub use session::{AskOutcome, AskPlan, MAX_FALLBACKS, ModelRef, RestMark, Session, ask_plan};
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
use proptest as _;
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
