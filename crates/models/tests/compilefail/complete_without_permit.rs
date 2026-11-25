use std::sync::Arc;

fn main() {
    let clock: Arc<dyn anybot_foundation::clock::Clock> =
        Arc::new(anybot_foundation::clock::SystemClock);
    let gateway = anybot_models::gateway(Arc::clone(&clock));
    let endpoint = anybot_models::resolve_endpoint(
        anybot_models::ProviderKind::Custom,
        "http://127.0.0.1:9/v1",
        "",
        "m",
        None,
    );
    let request = anybot_models::TurnRequest {
        system: String::new(),
        messages: Vec::new(),
        tools: Vec::new(),
        thinking: anybot_models::ThinkingMode::Auto,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    };
    let _sent = gateway.complete(&endpoint, &request, false);
    let _outside = anybot_models::Gateway::new(reqwest::Client::new(), clock);
}
