use std::future::Future;
use std::sync::Arc;

use crate::scripted::ScriptPoster;
use crate::session::{AskPlan, ModelRef, Session};
use crate::types::{ProviderEndpoint, ProviderKind, Role, ThinkingMode, TurnRequest, WireFormat};

pub(crate) fn drive<T>(future: impl Future<Output = T>) -> T {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(future)
}

pub(crate) fn hi() -> TurnRequest {
    TurnRequest {
        system: String::new(),
        messages: vec![crate::types::Message {
            role: Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking: ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}

pub(crate) fn endpoint(kind: ProviderKind, url: &str, key: &str, model: &str) -> ProviderEndpoint {
    crate::providers::resolve_endpoint(kind, url, key, model, None)
}

pub(crate) fn custom(model: &str, format: WireFormat) -> ProviderEndpoint {
    crate::providers::resolve_endpoint(
        ProviderKind::Custom,
        "http://127.0.0.1:9/v1",
        "",
        model,
        Some(format),
    )
}

pub(crate) fn model_ref(endpoint: ProviderEndpoint) -> ModelRef {
    ModelRef {
        endpoint,
        bot_context: None,
    }
}

pub(crate) fn ask_one(
    session: &mut Session,
    plan: &mut AskPlan<'_>,
) -> Result<crate::session::AskOutcome, crate::GatewayFail> {
    drive(session.ask(plan))
}

pub(crate) fn poster_of(steps: Vec<crate::scripted::ScriptStep>) -> Arc<ScriptPoster> {
    ScriptPoster::new(steps)
}
