use std::collections::BTreeSet;

use anybot_foundation::MessageRef;

use crate::assemble::{
    StepBlocks, bot_blocks, memory_blocks, prefix_blocks, prior_blocks, step_blocks, text_blocks,
    user_block,
};
use crate::collapse::{
    StepView, budget_collapse, model_swapped_after_last_built, plan_collapse, recorded_collapsed,
};
use crate::compile_must::block_summary;
use crate::estimate::{blocks_raw, calibrated, text_tokens};
use crate::input::BuildInput;
use crate::mask::{render_view, result_summary};
use crate::numbers::{
    K114_1_RECENT_STEPS, K114_4_MIN_VERBATIM_STEPS, K114_4_WATERMARK_MILLI,
    K114_6_MEMORY_CAP_MILLI, milli_of,
};
use crate::output::{
    Block, BlockKind, CollapseReason, Compiled, Insufficient, ManifestExcluded, NewEvent,
};
use crate::state::build_state;
use crate::state_render::fit_summary;

pub const CODE_MODEL_BUDGET: &str = "model-budget";

pub(crate) struct Stage {
    pub(crate) collapsed: BTreeSet<String>,
    pub(crate) events: Vec<NewEvent>,
    pub(crate) prior_keep: usize,
    pub(crate) memory_cap: u64,
    pub(crate) budget_collapse_done: bool,
}

pub fn compile(input: &BuildInput) -> Result<Compiled, anybot_foundation::FoundationError> {
    let usable = input.model.usable();
    let views = build_views(input);
    let recorded = recorded_collapsed(&input.events)?;
    let swap = model_swapped_after_last_built(&input.events);
    let (collapsed, events) =
        plan_collapse(&views, &recorded, usable, swap, input.compact_requested);
    let mut stage = Stage {
        collapsed,
        events,
        prior_keep: usize::MAX,
        memory_cap: milli_of(usable, K114_6_MEMORY_CAP_MILLI),
        budget_collapse_done: false,
    };
    for stage_no in 0..4 {
        let assembled = assemble(input, &stage);
        let total_raw = blocks_raw(&all_blocks(&assembled));
        let cal = calibrated(total_raw, &input.model.calibration);
        let under = cal <= milli_of(usable, K114_4_WATERMARK_MILLI);
        let send = if stage_no == 0 {
            cal <= usable
        } else if stage_no == 3 {
            under || cal <= usable
        } else {
            under
        };
        if send {
            return Ok(crate::compile_sent::sent(
                input, &assembled, &stage, usable, total_raw, cal,
            ));
        }
        apply_stage(stage_no, input, &views, &mut stage);
    }
    finish(input, &assemble(input, &stage), &stage, usable)
}

fn finish(
    input: &BuildInput,
    assembled: &Assembled,
    stage: &Stage,
    usable: u64,
) -> Result<Compiled, anybot_foundation::FoundationError> {
    let total_raw = blocks_raw(&all_blocks(assembled));
    let cal = calibrated(total_raw, &input.model.calibration);
    if cal > usable {
        let mandatory = crate::compile_must::mandatory_raw(assembled);
        let mandatory_cal = calibrated(mandatory, &input.model.calibration);
        let deficit = mandatory_cal.saturating_sub(usable);
        let deficit_i64 = u64_to_i64(deficit)?;
        return Ok(Compiled::Insufficient(Insufficient {
            deficit_tokens: deficit,
            usable,
            message: MessageRef::new(CODE_MODEL_BUDGET).with_int("deficit", deficit_i64),
        }));
    }
    Ok(crate::compile_sent::sent(
        input, assembled, stage, usable, total_raw, cal,
    ))
}

fn u64_to_i64(value: u64) -> Result<i64, anybot_foundation::FoundationError> {
    i64::try_from(value)
        .map_err(|_| anybot_foundation::FoundationError::IdParse("deficit too large".to_owned()))
}

fn apply_stage(step_no: usize, input: &BuildInput, views: &[StepView], stage: &mut Stage) {
    if step_no == 1 && !stage.budget_collapse_done {
        apply_budget_stage(views, stage);
    } else if step_no == 2 {
        apply_prior_stage(input, stage);
    } else if step_no == 3 {
        apply_memory_stage(input, stage);
    }
}

fn apply_prior_stage(input: &BuildInput, stage: &mut Stage) {
    let count = input.prior_chat.len();
    if count > K114_1_RECENT_STEPS {
        let split = count.saturating_sub(K114_1_RECENT_STEPS);
        let ids: Vec<String> = input
            .prior_chat
            .iter()
            .take(split)
            .map(|r| r.record_id.clone())
            .collect();
        stage.events.push(NewEvent::Collapsed {
            ids,
            reason: CollapseReason::Budget,
        });
    }
    stage.prior_keep = K114_1_RECENT_STEPS;
}

fn apply_memory_stage(input: &BuildInput, stage: &mut Stage) {
    let ids: Vec<String> = input.memories.iter().map(|m| m.id.clone()).collect();
    if !ids.is_empty() {
        stage.events.push(NewEvent::MemoryDropped { ids });
    }
    stage.memory_cap = 0;
}

fn apply_budget_stage(views: &[StepView], stage: &mut Stage) {
    stage.budget_collapse_done = true;
    let ids = budget_collapse(views, &stage.collapsed, K114_4_MIN_VERBATIM_STEPS);
    if ids.is_empty() {
        return;
    }
    stage.collapsed.extend(ids.iter().cloned());
    stage.events.push(NewEvent::Collapsed {
        ids,
        reason: CollapseReason::Budget,
    });
}

#[derive(Debug, Clone)]
pub(crate) struct Assembled {
    pub(crate) prefix: Vec<Block>,
    pub(crate) core: Vec<Block>,
    pub(crate) tail: Vec<Block>,
    pub(crate) excluded: Vec<ManifestExcluded>,
}

fn assemble(input: &BuildInput, stage: &Stage) -> Assembled {
    let builder = build_state(&input.events);
    let summary = fit_summary(&builder);
    let prefix = prefix_blocks(input);
    let mut core = Vec::new();
    core.extend(text_blocks(BlockKind::Constraint, &input.constraints));
    core.extend(crate::compile_must::must_blocks(input, &builder));
    core.extend(
        input
            .user_turns
            .iter()
            .map(|r| user_block(r, input.model.usable())),
    );
    core.extend(bot_blocks(&input.bot_turns));
    let steps = step_blocks(&input.steps, &stage.collapsed, input.view_limit);
    let memories = memory_blocks(&input.memories, stage.memory_cap);
    let mut excluded = join_excluded(&steps, &memories);
    excluded.extend(prior_excluded(&input.prior_chat, stage.prior_keep));
    core.extend(steps.blocks);
    core.extend(prior_blocks(&input.prior_chat, stage.prior_keep));
    let mut tail = vec![block_summary(&summary)];
    tail.extend(memories.blocks);
    Assembled {
        prefix,
        core,
        tail,
        excluded,
    }
}

fn prior_excluded(
    records: &[crate::input::ChatRecord],
    keep_recent: usize,
) -> Vec<ManifestExcluded> {
    if records.len() <= keep_recent {
        return Vec::new();
    }
    let split = records.len().saturating_sub(keep_recent);
    records
        .iter()
        .take(split)
        .map(|r| ManifestExcluded {
            source_id: r.record_id.clone(),
            reason: crate::output::ExcludedReason::Budget,
        })
        .collect()
}

fn join_excluded(steps: &StepBlocks, memories: &StepBlocks) -> Vec<ManifestExcluded> {
    let mut out: Vec<ManifestExcluded> = steps
        .excluded
        .iter()
        .map(|(id, reason)| ManifestExcluded {
            source_id: id.clone(),
            reason: *reason,
        })
        .collect();
    out.extend(
        memories
            .excluded
            .iter()
            .map(|(id, reason)| ManifestExcluded {
                source_id: id.clone(),
                reason: *reason,
            }),
    );
    out
}

fn all_blocks(a: &Assembled) -> Vec<Block> {
    a.prefix
        .iter()
        .chain(a.core.iter())
        .chain(a.tail.iter())
        .cloned()
        .collect()
}

fn build_views(input: &BuildInput) -> Vec<StepView> {
    input
        .steps
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let full = render_view(s, input.view_limit);
            let masked = format!("{} {}", result_summary(s), s.source_ref);
            crate::collapse::step_view(s, idx, text_tokens(&full), text_tokens(&masked))
        })
        .collect()
}
