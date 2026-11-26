use serde::{Deserialize, Serialize};

use anybot_foundation::MessageRef;
use anybot_models::{Capability, Role, ThinkingMode, ToolSpec, TurnRequest};

use crate::input::ModelPlan;
use crate::output_turn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    System,
    ToolDoc,
    Pinned,
    UserText,
    UserRef,
    BotText,
    Constraint,
    Criterion,
    CurrentStep,
    OpenQuestion,
    PendingAction,
    ToolIntent,
    ToolResultFull,
    ToolResultMasked,
    ToolResultDuplicate,
    PriorChatFull,
    PriorChatRef,
    StateSummary,
    Memory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockForm {
    Verbatim,
    Reference,
    Masked,
    Summary,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    pub form: BlockForm,
    pub source_ids: Vec<String>,
    pub text: String,
    pub estimate_raw: u64,
    pub call_id: Option<String>,
    pub tool_name: Option<String>,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledRequest {
    pub system: String,
    pub blocks: Vec<Block>,
    pub prefix_len: usize,
}

impl CompiledRequest {
    pub fn to_turn_request(
        &self,
        tools: Vec<ToolSpec>,
        thinking: ThinkingMode,
        max_output_tokens: Option<u64>,
    ) -> Result<TurnRequest, anybot_foundation::FoundationError> {
        Ok(TurnRequest {
            system: output_turn::join_system(self),
            messages: output_turn::turn_messages(self)?,
            tools,
            thinking,
            max_output_tokens,
            attachments: Vec::new(),
            cache_prefix: true,
            estimate_override: None,
        })
    }

    #[must_use]
    pub fn prefix(&self) -> Vec<Block> {
        self.blocks.iter().take(self.prefix_len).cloned().collect()
    }

    pub fn prefix_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self.prefix())
    }
}

impl BlockKind {
    #[must_use]
    pub fn role(self) -> Role {
        output_turn::block_role(self)
    }
}

pub const CODE_TURN_PROGRAM_NOTE: &str = "turn-program-note";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExcludedReason {
    Budget,
    Masked,
    Duplicate,
    Irrelevant,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub kind: BlockKind,
    pub form: BlockForm,
    pub source_ids: Vec<String>,
    pub estimate_raw: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestExcluded {
    pub source_id: String,
    pub reason: ExcludedReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestBudget {
    pub usable: u64,
    pub reserved_output: u64,
    pub estimate_raw: u64,
    pub estimate_calibrated: u64,
    pub margin_milli: u64,
    pub watermark_milli: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestModel {
    pub provider: String,
    pub model: String,
    pub estimator: String,
    pub calibration_samples: usize,
    pub context_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub included: Vec<ManifestEntry>,
    pub excluded: Vec<ManifestExcluded>,
    pub budget: ManifestBudget,
    pub model: ManifestModel,
    pub actual_input_tokens: Option<u64>,
    pub estimate_delta: Option<i64>,
}

impl Manifest {
    pub fn record_actual(
        &mut self,
        actual_input_tokens: u64,
    ) -> Result<(), anybot_foundation::FoundationError> {
        self.actual_input_tokens = Some(actual_input_tokens);
        let raw = i64::try_from(self.budget.estimate_calibrated).map_err(|_| {
            anybot_foundation::FoundationError::IdParse("estimate too large".to_owned())
        })?;
        let actual = i64::try_from(actual_input_tokens).map_err(|_| {
            anybot_foundation::FoundationError::IdParse("actual too large".to_owned())
        })?;
        self.estimate_delta = Some(actual.saturating_sub(raw));
        Ok(())
    }
}

#[must_use]
pub fn manifest_model(plan: &ModelPlan) -> ManifestModel {
    let capability: &Capability = &plan.capability;
    ManifestModel {
        provider: format!("{:?}", capability.provider),
        model: capability.model.clone(),
        estimator: "formula:text_tokens".to_owned(),
        calibration_samples: plan.calibration.samples(),
        context_tokens: capability.context_tokens,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollapseReason {
    Accumulated,
    Budget,
    Compact,
    ModelSwapped,
    ReplaceAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewEvent {
    ContextBuilt,
    Collapsed {
        ids: Vec<String>,
        reason: CollapseReason,
    },
    MemoryDropped {
        ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Insufficient {
    pub deficit_tokens: u64,
    pub usable: u64,
    pub message: MessageRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compiled {
    Sent {
        request: CompiledRequest,
        manifest: Box<Manifest>,
        events: Vec<NewEvent>,
    },
    Insufficient(Insufficient),
}

impl Compiled {
    #[must_use]
    pub fn is_sent(&self) -> bool {
        matches!(self, Compiled::Sent { .. })
    }
}
