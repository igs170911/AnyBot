use std::collections::BTreeMap;

use crate::estimate::text_tokens;
use crate::input::{BuildInput, ChatRecord, StepRecord};
use crate::mask::{duplicate_line, masked_line, result_status};
use crate::numbers::{
    K114_5_REF_HEAD_CHARS, K114_5_USER_REF_MILLI, K114_6_MEMORY_MIN_KEEP, milli_of,
};
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
pub fn prefix_blocks(input: &BuildInput) -> Vec<Block> {
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
    for (idx, item) in input.pinned.items.iter().enumerate() {
        out.push(block(
            BlockKind::Pinned,
            BlockForm::Verbatim,
            vec![format!("pinned-{idx}")],
            item.clone(),
        ));
    }
    out
}

#[must_use]
pub fn user_block(record: &ChatRecord, usable: u64) -> Block {
    let threshold = milli_of(usable, K114_5_USER_REF_MILLI);
    let tokens = text_tokens(&record.text);
    if tokens <= threshold {
        return block(
            BlockKind::UserText,
            BlockForm::Verbatim,
            vec![record.record_id.clone()],
            record.text.clone(),
        );
    }
    let head: String = record.text.chars().take(K114_5_REF_HEAD_CHARS).collect();
    let text = format!(
        "ref {} len {}: head: {}",
        record.record_id,
        record.text.chars().count(),
        head
    );
    block(
        BlockKind::UserRef,
        BlockForm::Reference,
        vec![record.record_id.clone()],
        text,
    )
}

#[must_use]
pub fn text_blocks(kind: BlockKind, items: &[String]) -> Vec<Block> {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            block(
                kind,
                BlockForm::Verbatim,
                vec![format!("{kind:?}-{idx}")],
                item.clone(),
            )
        })
        .collect()
}

#[must_use]
pub fn bot_blocks(turns: &[ChatRecord]) -> Vec<Block> {
    turns
        .iter()
        .map(|t| {
            block(
                BlockKind::BotText,
                BlockForm::Verbatim,
                vec![t.record_id.clone()],
                t.text.clone(),
            )
        })
        .collect()
}

#[must_use]
pub fn prior_blocks(records: &[ChatRecord], keep_recent: usize) -> Vec<Block> {
    if records.len() <= keep_recent {
        return chat_blocks(records);
    }
    let split = records.len().saturating_sub(keep_recent);
    let (old, recent) = records.split_at(split);
    let mut out = vec![ref_block(old)];
    out.extend(chat_blocks(recent));
    out
}

fn chat_blocks(records: &[ChatRecord]) -> Vec<Block> {
    records
        .iter()
        .map(|r| {
            block(
                BlockKind::PriorChatFull,
                BlockForm::Verbatim,
                vec![r.record_id.clone()],
                r.text.clone(),
            )
        })
        .collect()
}

fn ref_block(old: &[ChatRecord]) -> Block {
    let joined: String = old.iter().fold(String::new(), |mut acc, r| {
        acc.push_str(&r.text);
        acc.push(' ');
        acc
    });
    let head: String = joined.chars().take(K114_5_REF_HEAD_CHARS).collect();
    let ids: Vec<String> = old.iter().map(|r| r.record_id.clone()).collect();
    let text = format!("ref chat len {} head: {}", joined.chars().count(), head);
    block(BlockKind::PriorChatRef, BlockForm::Reference, ids, text)
}

pub struct StepBlocks {
    pub blocks: Vec<Block>,
    pub excluded: Vec<(String, ExcludedReason)>,
}

#[must_use]
pub fn step_blocks(
    steps: &[StepRecord],
    collapsed: &std::collections::BTreeSet<String>,
    view_limit: usize,
) -> StepBlocks {
    let mut first_seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut blocks = Vec::new();
    let mut excluded = Vec::new();
    for (idx, step) in steps.iter().enumerate() {
        blocks.push(intent_block(step, idx));
        push_result(
            step,
            idx,
            collapsed,
            view_limit,
            &mut first_seen,
            &mut blocks,
            &mut excluded,
        );
    }
    StepBlocks { blocks, excluded }
}

fn intent_block(step: &StepRecord, idx: usize) -> Block {
    let id = format!("call-{}", step.call_id.as_str());
    let text = format!(
        "step {idx}: call {} args {} status {}",
        step.tool,
        step.args_summary,
        result_status(step)
    );
    block(BlockKind::ToolIntent, BlockForm::Verbatim, vec![id], text)
}

fn push_result(
    step: &StepRecord,
    idx: usize,
    collapsed: &std::collections::BTreeSet<String>,
    view_limit: usize,
    first_seen: &mut BTreeMap<String, usize>,
    blocks: &mut Vec<Block>,
    excluded: &mut Vec<(String, ExcludedReason)>,
) {
    let id = format!("result-{}", step.source_ref);
    if !step.has_output() {
        return;
    }
    let digest = crate::digest::content_digest(&step.content);
    if let Some(first) = first_seen.get(&digest) {
        blocks.push(block(
            BlockKind::ToolResultDuplicate,
            BlockForm::Duplicate,
            vec![id.clone()],
            duplicate_line(step, *first),
        ));
        excluded.push((id.clone(), ExcludedReason::Duplicate));
        return;
    }
    first_seen.insert(digest, idx);
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
    StepBlocks { blocks, excluded }
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
