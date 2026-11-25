use anybot_models::ProviderKind;

pub(crate) fn endpoint(
    kind: ProviderKind,
    port: u16,
    model: &str,
    key: &str,
) -> anybot_models::ProviderEndpoint {
    let format = if matches!(kind, ProviderKind::Anthropic) {
        Some(anybot_models::WireFormat::Anthropic)
    } else {
        Some(anybot_models::WireFormat::OpenAiCompatible)
    };
    anybot_models::resolve_endpoint(
        kind,
        &format!("http://127.0.0.1:{port}/v1"),
        key,
        model,
        format,
    )
}

pub(crate) fn hi(thinking: anybot_models::ThinkingMode) -> anybot_models::TurnRequest {
    anybot_models::TurnRequest {
        system: String::new(),
        messages: vec![anybot_models::Message {
            role: anybot_models::Role::User,
            text: "hi".to_owned(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        tools: Vec::new(),
        thinking,
        max_output_tokens: None,
        attachments: Vec::new(),
        cache_prefix: false,
        estimate_override: None,
    }
}
