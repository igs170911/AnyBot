pub mod assemble;
pub mod collapse;
pub mod compile;
mod compile_must;
mod compile_sent;
#[cfg(test)]
mod compile_tests_support;
pub mod digest;
pub mod estimate;
#[cfg(test)]
mod fixtures;
pub mod input;
pub mod mask;
pub mod numbers;
pub mod output;
pub mod read;
pub mod state;
pub mod state_render;

use anybot_domain as _;

pub use assemble::block;
pub use collapse::{StepView, recorded_collapsed};
pub use compile::compile;
pub use digest::content_digest;
pub use estimate::{blocks_raw, calibrated, text_tokens, under_watermark};
pub use input::{
    BuildInput, ChatRecord, MemoryRecord, MemorySource, ModelPlan, PinnedSnapshot, StepOutcomeKind,
    StepRecord,
};
pub use mask::{masked_line, render_view, result_summary};
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
pub use state::build_state;
pub use state_render::{StateSummary, fit_summary};

#[cfg(test)]
#[path = "budget_extra_tests.rs"]
mod budget_extra_tests;
#[cfg(test)]
#[path = "budget_tests.rs"]
mod budget_tests;
#[cfg(test)]
#[path = "build_tests.rs"]
mod build_tests;

#[cfg(test)]
#[path = "build_unit_tests.rs"]
mod build_unit_tests;
#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
#[cfg(test)]
#[path = "mask_pair_tests.rs"]
mod mask_pair_tests;
#[cfg(test)]
#[path = "mask_tests.rs"]
mod mask_tests;

#[cfg(test)]
#[path = "mask_prop_tests.rs"]
mod mask_prop_tests;
#[cfg(test)]
#[path = "must_tests.rs"]
mod must_tests;

#[cfg(test)]
use proptest as _;
#[cfg(test)]
use serde_json as _;
