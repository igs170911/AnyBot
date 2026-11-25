use crate::providers::CLAUDE_MAX_OUTPUT;
use crate::types::{ProviderKind, WireFormat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TokenizerKind {
    Formula,
    O200k,
    TokenizerJson,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Capability {
    pub provider: ProviderKind,
    pub model: String,
    pub format: WireFormat,
    pub context_tokens: u64,
    pub max_output_tokens: u64,
    pub tools: bool,
    pub stream: bool,
    pub thinking: bool,
    pub vision: bool,
    pub cache: bool,
    pub tokenizer: TokenizerKind,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Learned {
    pub key: String,
    pub context_tokens: Option<u64>,
    pub tools: Option<bool>,
    pub thinking: Option<bool>,
    pub vision: Option<bool>,
    pub include_usage: Option<bool>,
    pub calibration: crate::estimate::Calibration,
}

impl Learned {
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            context_tokens: None,
            tools: None,
            thinking: None,
            vision: None,
            include_usage: None,
            calibration: crate::estimate::Calibration::new(),
        }
    }

    pub fn note_too_long(&mut self, limit: u64) {
        self.context_tokens = Some(match self.context_tokens {
            Some(current) => current.min(limit),
            None => limit,
        });
    }
}

#[must_use]
pub fn model_key(provider: ProviderKind, base_url: &str, model: &str) -> String {
    format!("{provider:?}|{base_url}|{model}")
}

#[must_use]
pub fn builtin(provider: ProviderKind, model: &str) -> Option<Capability> {
    SEEDS
        .iter()
        .find(|row| row.provider == provider && row.model == model)
        .map(seed_to_row)
}

#[must_use]
pub fn resolve(
    provider: ProviderKind,
    model: &str,
    bot_context: Option<u64>,
    learned: Option<&Learned>,
) -> Capability {
    let mut row = match builtin(provider, model) {
        Some(found) => found,
        None => unknown(provider, model, bot_context),
    };
    if let Some(limit) = bot_context {
        row.context_tokens = row.context_tokens.min(limit);
    }
    if let Some(learned) = learned {
        if let Some(limit) = learned.context_tokens {
            row.context_tokens = row.context_tokens.min(limit);
        }
        if let Some(tools) = learned.tools {
            row.tools = tools;
        }
        if let Some(thinking) = learned.thinking {
            row.thinking = thinking;
        }
        if let Some(vision) = learned.vision {
            row.vision = vision;
        }
    }
    if matches!(provider, ProviderKind::Anthropic) {
        row.max_output_tokens = row.max_output_tokens.min(CLAUDE_MAX_OUTPUT);
    }
    row
}

fn unknown(provider: ProviderKind, model: &str, bot_context: Option<u64>) -> Capability {
    Capability {
        provider,
        model: model.to_owned(),
        format: crate::providers::defaults_for(provider).format,
        context_tokens: bot_context.unwrap_or(8_192),
        max_output_tokens: 8_192,
        tools: true,
        stream: true,
        thinking: true,
        vision: true,
        cache: false,
        tokenizer: TokenizerKind::Formula,
        source: "models-capabilities S2.3".to_owned(),
    }
}

#[must_use]
pub fn usable_context(
    table: u64,
    bot_limit: Option<u64>,
    learned_limit: Option<u64>,
    reserved_output: u64,
) -> u64 {
    let mut usable = table;
    if let Some(limit) = bot_limit {
        usable = usable.min(limit);
    }
    if let Some(limit) = learned_limit {
        usable = usable.min(limit);
    }
    usable.saturating_sub(reserved_output)
}

struct Seed {
    provider: ProviderKind,
    model: &'static str,
    format: WireFormat,
    context_tokens: u64,
    max_output_tokens: u64,
    cache: bool,
    tokenizer: TokenizerKind,
    source: &'static str,
}

const SEEDS: &[Seed] = &[
    Seed {
        provider: ProviderKind::Xai,
        model: "grok-4",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 256_000,
        max_output_tokens: 16_384,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 name; context not confirmed against xAI docs this milestone",
    },
    Seed {
        provider: ProviderKind::OpenAi,
        model: "gpt-5",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 400_000,
        max_output_tokens: 128_000,
        cache: true,
        tokenizer: TokenizerKind::O200k,
        source: "spec/03c section 13 name; tokenizer o200k per models-capabilities S5; context not confirmed against OpenAI docs this milestone",
    },
    Seed {
        provider: ProviderKind::Anthropic,
        model: "claude-opus-5-5",
        format: WireFormat::Anthropic,
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 name; max output 64000 per models-providers S4; context not confirmed against Anthropic docs this milestone",
    },
    Seed {
        provider: ProviderKind::Anthropic,
        model: "claude-sonnet-5-5",
        format: WireFormat::Anthropic,
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 extra name; max output capped at 64000",
    },
    Seed {
        provider: ProviderKind::Anthropic,
        model: "claude-haiku-5-5",
        format: WireFormat::Anthropic,
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 extra name; max output capped at 64000",
    },
    Seed {
        provider: ProviderKind::Anthropic,
        model: "claude-fable-5-1",
        format: WireFormat::Anthropic,
        context_tokens: 200_000,
        max_output_tokens: 64_000,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 extra name; max output capped at 64000",
    },
    Seed {
        provider: ProviderKind::Gemini,
        model: "gemini-2.5-flash",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 1_048_576,
        max_output_tokens: 65_536,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 name; context not confirmed against Gemini docs this milestone",
    },
    Seed {
        provider: ProviderKind::OpenRouter,
        model: "openrouter/auto",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 128_000,
        max_output_tokens: 8_192,
        cache: false,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13; OpenRouter auto has no fixed context, placeholder 128000",
    },
    Seed {
        provider: ProviderKind::OpenCodeGo,
        model: "kimi-k3",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 128_000,
        max_output_tokens: 8_192,
        cache: false,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 name; format guessed OpenAI-compatible",
    },
    Seed {
        provider: ProviderKind::Ollama,
        model: "llama3.2",
        format: WireFormat::OpenAiCompatible,
        context_tokens: 128_000,
        max_output_tokens: 8_192,
        cache: false,
        tokenizer: TokenizerKind::Formula,
        source: "spec/03c section 13 name; local context depends on the pulled model",
    },
];

#[must_use]
pub fn builtin_rows() -> Vec<Capability> {
    SEEDS.iter().map(seed_to_row).collect()
}

fn seed_to_row(seed: &Seed) -> Capability {
    Capability {
        provider: seed.provider,
        model: seed.model.to_owned(),
        format: seed.format,
        context_tokens: seed.context_tokens,
        max_output_tokens: seed.max_output_tokens,
        tools: true,
        stream: true,
        thinking: true,
        vision: true,
        cache: seed.cache,
        tokenizer: seed.tokenizer,
        source: seed.source.to_owned(),
    }
}
