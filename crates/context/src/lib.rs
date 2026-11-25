pub mod digest;
pub mod input;

use anybot_domain as _;
pub mod numbers;
pub mod output;

pub use digest::content_digest;
pub use input::{
    BuildInput, ChatRecord, MemoryRecord, MemorySource, ModelPlan, PinnedSnapshot, StepOutcomeKind,
    StepRecord,
};
pub use numbers::{
    K114_1_RECENT_STEPS, K114_2_DEFAULT_RESULT_CHARS, K114_2_ERROR_WINDOW_THIRDS,
    K114_3_ACCUMULATE_MARGIN_MILLI, K114_3_SHORT_RESULT_CHARS, K114_4_MIN_VERBATIM_STEPS,
    K114_4_WATERMARK_MILLI, K114_5_REF_HEAD_CHARS, K114_5_USER_REF_MILLI, K114_6_MEMORY_CAP_MILLI,
    K114_6_MEMORY_MIN_KEEP, K114_7_STATE_MAX_TOKENS, milli_of,
};
pub use output::{
    Block, BlockForm, BlockKind, CollapseReason, Compiled, CompiledRequest, ExcludedReason,
    Insufficient, Manifest, ManifestBudget, ManifestEntry, ManifestExcluded, ManifestModel,
    NewEvent, manifest_model,
};

#[cfg(test)]
use proptest as _;
#[cfg(test)]
use serde_json as _;
