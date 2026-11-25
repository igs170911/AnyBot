use std::collections::BTreeMap;

use crate::chat::text_blocks;
use crate::estimate::text_tokens;
use crate::input::{BuildInput, StepRecord};
use crate::mask::{duplicate_line, masked_line, result_status};
use crate::numbers::K114_6_MEMORY_MIN_KEEP;
use crate::output::{Block, BlockForm, BlockKind, ExcludedReason};

#[must_use]
pub fn block(kind: BlockKind, form: BlockForm, source_ids: Vec<String>, text: String) -> Block {
    let estimate_raw = text_tokens(&text);
    Block {
        kind,
        form,
        source_ids,
        text,
        estimate_raw,
    }
}

#[must_use]
pub fn prefix_blocks(input: &BuildInput, criteria: &[Block]) -> Vec<Block> {
    let mut out = vec![block(
        BlockKind::System,
        BlockForm::Verbatim,
        Vec::new(),
        input.system_prompt.clone(),
    )];
    for spec in &input.tool_specs {
        out.push(block(
            BlockKind::ToolDoc,
            BlockForm::Verbatim,
            vec![spec.name.clone()],
            format!("{}: {}", spec.name, spec.description),
        ));
    }
    out.push(block(
        BlockKind::Pinned,
        BlockForm::Verbatim,
        vec![format!("pinned-v{}", input.pinned.version)],
        format!("pinned snapshot v{}", input.pinned.version),
    ));
    for (idx, item) in input.pinned.items.iter().enumerate() {
        out.push(block(
            BlockKind::Pinned,
            BlockForm::Verbatim,
            vec![format!("pinned-{idx}")],
            item.clone(),
        ));
    }
    out.extend(text_blocks(BlockKind::Constraint, &input.constraints));
    out.extend(criteria.to_vec());
    out
}

pub struct StepBlocks {
    pub groups: Vec<Vec<Block>>,
    pub excluded: Vec<(String, ExcludedReason)>,
}

#[must_use]
pub fn step_blocks(
    steps: &[StepRecord],
    collapsed: &std::collections::BTreeSet<String>,
    view_limit: usize,
) -> StepBlocks {
    let mut first_seen: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut groups = Vec::new();
    let mut excluded = Vec::new();
    for (idx, step) in steps.iter().enumerate() {
        let mut group = vec![intent_block(step, idx)];
        push_result(
            step,
            idx,
            collapsed,
            view_limit,
            &mut first_seen,
            &mut group,
            &mut excluded,
        );
        groups.push(group);
    }
    StepBlocks { groups, excluded }
}

fn intent_block(step: &StepRecord, idx: usize) -> Block {
    let id = format!("call-{}", step.call_id.as_str());
    let text = format!(
        "step {idx}: call {} args {} level {:?} status {}",
        step.tool,
        step.args_summary,
        step.level,
        result_status(step)
    );
    block(BlockKind::ToolIntent, BlockForm::Verbatim, vec![id], text)
}

fn push_result(
    step: &StepRecord,
    idx: usize,
    collapsed: &std::collections::BTreeSet<String>,
    view_limit: usize,
    first_seen: &mut BTreeMap<String, (usize, String)>,
    blocks: &mut Vec<Block>,
    excluded: &mut Vec<(String, ExcludedReason)>,
) {
    let id = format!("result-{}", step.source_ref);
    if !step.has_output() {
        return;
    }
    let digest = crate::digest::content_digest(&step.content);
    if let Some((first, first_ref)) = first_seen.get(&digest) {
        if collapsed.contains(first_ref) {
            let full = crate::mask::render_view(step, view_limit);
            blocks.push(block(
                BlockKind::ToolResultFull,
                BlockForm::Verbatim,
                vec![id.clone()],
                full,
            ));
            return;
        }
        blocks.push(block(
            BlockKind::ToolResultDuplicate,
            BlockForm::Duplicate,
            vec![id.clone()],
            duplicate_line(step, *first),
        ));
        excluded.push((id.clone(), ExcludedReason::Duplicate));
        return;
    }
    first_seen.insert(digest, (idx, step.source_ref.clone()));
    if collapsed.contains(&step.source_ref) && !step.keeps_full() {
        blocks.push(block(
            BlockKind::ToolResultMasked,
            BlockForm::Masked,
            vec![id.clone()],
            masked_line(step),
        ));
        excluded.push((id.clone(), ExcludedReason::Masked));
        return;
    }
    let full = crate::mask::render_view(step, view_limit);
    blocks.push(block(
        BlockKind::ToolResultFull,
        BlockForm::Verbatim,
        vec![id.clone()],
        full,
    ));
}

#[must_use]
pub fn memory_blocks(memories: &[crate::input::MemoryRecord], cap: u64) -> StepBlocks {
    let mut sorted: Vec<&crate::input::MemoryRecord> = memories.iter().collect();
    sorted.sort_by(|a, b| {
        b.relevance
            .cmp(&a.relevance)
            .then_with(|| a.candidate.cmp(&b.candidate))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut blocks = Vec::new();
    let mut excluded = Vec::new();
    let mut used = 0u64;
    for (idx, m) in sorted.iter().enumerate() {
        let text = format!("memory{}: {}", marker(m), m.content);
        let tokens = text_tokens(&text);
        let over = used.saturating_add(tokens) > cap && idx >= K114_6_MEMORY_MIN_KEEP;
        if over {
            excluded.push((m.id.clone(), ExcludedReason::Budget));
            continue;
        }
        used = used.saturating_add(tokens);
        blocks.push(block(
            BlockKind::Memory,
            BlockForm::Verbatim,
            vec![m.id.clone()],
            text,
        ));
    }
    StepBlocks {
        groups: vec![blocks],
        excluded,
    }
}

impl StepBlocks {
    #[must_use]
    pub fn all(&self) -> Vec<&Block> {
        self.groups.iter().flatten().collect()
    }
}

fn marker(m: &crate::input::MemoryRecord) -> &'static str {
    if m.candidate {
        return " [candidate]";
    }
    if m.source == crate::input::MemorySource::ModelInference {
        return " [inference]";
    }
    ""
}
