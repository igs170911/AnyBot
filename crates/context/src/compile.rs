use std::collections::BTreeSet;

use anybot_foundation::MessageRef;

use crate::assemble::{StepBlocks, memory_kept, prefix_blocks, step_blocks};
use crate::chat::{bot_block, prior_blocks, user_block};
use crate::collapse::{
    StepView, model_swapped_after_last_built, plan_collapse, recorded_collapsed,
};
use crate::compile_must::block_summary;
use crate::estimate::{blocks_raw, calibrated, text_tokens};
use crate::input::BuildInput;
use crate::mask::{render_view, result_summary};
use crate::numbers::{K114_4_WATERMARK_MILLI, K114_6_MEMORY_CAP_MILLI, milli_of};
use crate::output::{Block, CollapseReason, Compiled, Insufficient, ManifestExcluded, NewEvent};
use crate::state::build_state;
use crate::state_render::fit_summary;

pub const CODE_MODEL_BUDGET: &str = "model-budget";

pub(crate) struct Stage {
    pub(crate) collapsed: BTreeSet<String>,
    pub(crate) events: Vec<NewEvent>,
    pub(crate) prior_keep: usize,
    pub(crate) memory_cap: u64,
    pub(crate) dropped_results: Vec<String>,
    pub(crate) dropped_prior: Vec<String>,
    pub(crate) dropped_memory: Vec<String>,
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
        dropped_results: Vec::new(),
        dropped_prior: Vec::new(),
        dropped_memory: Vec::new(),
    };
    if let Some(done) = send_when_fits(input, &mut stage, usable, true) {
        return Ok(done);
    }
    trim_until_watermark(input, &views, &mut stage, usable);
    finish(input, &assemble(input, &stage), &stage, usable)
}

fn send_when_fits(
    input: &BuildInput,
    stage: &mut Stage,
    usable: u64,
    fits_is_enough: bool,
) -> Option<Compiled> {
    let assembled = assemble(input, stage);
    let total_raw = blocks_raw(&all_blocks(&assembled));
    let cal = calibrated(total_raw, &input.model.calibration);
    let watermark = milli_of(usable, K114_4_WATERMARK_MILLI);
    let ready = cal <= watermark || (fits_is_enough && cal <= usable);
    if !ready {
        return None;
    }
    Some(crate::compile_sent::sent(
        input, &assembled, stage, usable, total_raw, cal,
    ))
}

fn trim_until_watermark(input: &BuildInput, views: &[StepView], stage: &mut Stage, usable: u64) {
    for item in 0..3 {
        trim_one_item(item, input, views, stage, usable);
        if send_when_fits(input, stage, usable, false).is_some() {
            record_drops(stage);
            return;
        }
    }
    record_drops(stage);
}

fn trim_one_item(
    item: usize,
    input: &BuildInput,
    views: &[StepView],
    stage: &mut Stage,
    usable: u64,
) {
    loop {
        if !drop_one(item, input, views, stage) {
            return;
        }
        if send_when_fits(input, stage, usable, false).is_some() {
            return;
        }
    }
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

fn drop_one(item: usize, input: &BuildInput, views: &[StepView], stage: &mut Stage) -> bool {
    match item {
        0 => crate::compile_trim::drop_oldest_result(views, stage),
        1 => crate::compile_trim::drop_oldest_prior(input, stage),
        _ => crate::compile_trim::drop_oldest_memory(input, stage),
    }
}

fn record_drops(stage: &mut Stage) {
    let results = stage.dropped_results.clone();
    let prior = stage.dropped_prior.clone();
    let memory = stage.dropped_memory.clone();
    push_collapsed(&results, stage);
    push_collapsed(&prior, stage);
    if memory.is_empty() {
        return;
    }
    stage.events.push(NewEvent::MemoryDropped { ids: memory });
}

fn push_collapsed(ids: &[String], stage: &mut Stage) {
    if ids.is_empty() {
        return;
    }
    stage.events.push(NewEvent::Collapsed {
        ids: ids.to_vec(),
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
    let criteria = crate::compile_must::criteria_blocks(&builder);
    let prefix = prefix_blocks(input, &criteria);
    let steps = step_blocks(&input.steps, &stage.collapsed, input.view_limit);
    let memories = memory_kept(&input.memories, stage.memory_cap, &stage.dropped_memory);
    let mut excluded = join_excluded(&steps, &memories);
    excluded.extend(prior_excluded(&input.prior_chat, stage.prior_keep));
    let core = core_blocks(input, stage, &steps);
    let mut tail = vec![block_summary(&summary)];
    tail.extend(crate::compile_must::tail_blocks(input, &builder));
    tail.extend(memories.all().into_iter().cloned());
    Assembled {
        prefix,
        core,
        tail,
        excluded,
    }
}

fn core_blocks(input: &BuildInput, stage: &Stage, steps: &StepBlocks) -> Vec<Block> {
    let mut timed: Vec<(u64, u8, Vec<Block>)> = Vec::new();
    for (idx, step) in input.steps.iter().enumerate() {
        if let Some(group) = steps.groups.get(idx) {
            timed.push((step.seq, 1, group.clone()));
        }
    }
    for r in &input.user_turns {
        timed.push((r.seq, 2, vec![user_block(r, input.model.usable())]));
    }
    for r in &input.bot_turns {
        timed.push((r.seq, 3, vec![bot_block(r)]));
    }
    for (seq, group) in prior_blocks(&input.prior_chat, stage.prior_keep) {
        timed.push((seq, 4, group));
    }
    timed.sort_by_key(|(seq, group, _)| (*seq, *group));
    timed
        .into_iter()
        .flat_map(|(_, _, blocks)| blocks)
        .collect()
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
