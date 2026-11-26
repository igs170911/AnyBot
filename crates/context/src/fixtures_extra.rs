use proptest::prelude::*;

use crate::input::{MemoryRecord, MemorySource, ModelPlan};
use anybot_models::{Calibration, Capability, ProviderKind, TokenizerKind, ToolSpec, WireFormat};

pub(crate) fn memory(index: usize) -> BoxedStrategy<MemoryRecord> {
    (0u32..100, prop::bool::ANY)
        .prop_map(move |(relevance, candidate)| MemoryRecord {
            id: format!("m{index}"),
            content: format!("memory {index}"),
            relevance,
            source: MemorySource::ModelInference,
            candidate,
        })
        .boxed()
}

pub(crate) fn model_plan(context_low: u64, context_high: u64) -> BoxedStrategy<ModelPlan> {
    (context_low..context_high, 0u64..1_000, 0u64..8_000)
        .prop_map(|(context, reserved, output_cap)| ModelPlan {
            capability: Capability {
                provider: ProviderKind::OpenAi,
                model: "m".to_owned(),
                format: WireFormat::OpenAiCompatible,
                context_tokens: context,
                max_output_tokens: output_cap,
                tools: true,
                stream: true,
                thinking: false,
                vision: false,
                cache: true,
                tokenizer: TokenizerKind::Formula,
                source: "t".to_owned(),
            },
            calibration: Calibration::new(),
            reserved_output: reserved,
        })
        .boxed()
}

pub(crate) fn tool_spec_at(index: usize) -> ToolSpec {
    ToolSpec {
        name: format!("spec{index}"),
        description: format!("spec {index} description"),
        parameters: serde_json::json!({ "type": "object" }),
    }
}

pub(crate) fn context_built_kind() -> anybot_protocol::KnownTaskEventKind {
    anybot_protocol::KnownTaskEventKind::ContextBuilt {
        manifest_id: "m".to_owned(),
        model: anybot_protocol::ModelName {
            provider: "OpenAi".to_owned(),
            base_url: "http://127.0.0.1:9/v1".to_owned(),
            model: "m".to_owned(),
        },
        estimate_tokens: 1,
    }
}
