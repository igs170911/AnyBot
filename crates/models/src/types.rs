use anybot_foundation::Secret;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Xai,
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
    OpenCodeGo,
    Ollama,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireFormat {
    OpenAiCompatible,
    Anthropic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinkingMode {
    Auto,
    Fast,
    Balanced,
    Deep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolArguments {
    Json(serde_json::Value),
    Broken(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: ToolArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAttachment {
    pub provider: ProviderKind,
    pub model: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnRequest {
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub thinking: ThinkingMode,
    pub max_output_tokens: Option<u64>,
    pub attachments: Vec<NativeAttachment>,
    pub cache_prefix: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopKind {
    Finished,
    NeedsTool,
    Length,
    Refusal,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub thinking_tokens: u64,
}

impl Usage {
    #[must_use]
    pub fn input_total(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.cache_read_tokens)
            .saturating_add(self.cache_write_tokens)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageReport {
    Absent,
    Present(Usage),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TurnResult {
    pub text: String,
    pub thinking: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: UsageReport,
    pub stop: StopKind,
    pub attachment: Option<NativeAttachment>,
    pub model: String,
    pub provider: ProviderKind,
}

#[derive(Debug)]
pub struct ProviderEndpoint {
    pub kind: ProviderKind,
    pub base_url: String,
    pub api_key: Option<Secret>,
    pub model: String,
    pub format: WireFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    Watching,
    Unattended,
}

impl TurnResult {
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.text.is_empty() && self.tool_calls.is_empty()
    }
}
