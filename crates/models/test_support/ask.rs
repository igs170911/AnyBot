use anybot_models::{Message, Role, ThinkingMode, TurnRequest};
use proptest as _;

pub(crate) fn hi(thinking: ThinkingMode) -> TurnRequest {
    TurnRequest {
        system: String::new(),
        messages: vec![Message {
            role: Role::User,
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
