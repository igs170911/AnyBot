use crate::assemble::block;

use crate::estimate::text_tokens;
use crate::input::ChatRecord;
use crate::numbers::{K114_5_REF_HEAD_CHARS, K114_5_USER_REF_MILLI, milli_of};
use crate::output::{Block, BlockForm, BlockKind};

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
pub fn bot_block(record: &ChatRecord) -> Block {
    block(
        BlockKind::BotText,
        BlockForm::Verbatim,
        vec![record.record_id.clone()],
        record.text.clone(),
    )
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
pub fn prior_blocks(records: &[ChatRecord], keep_recent: usize) -> Vec<(u64, Vec<Block>)> {
    let mut sorted: Vec<&ChatRecord> = records.iter().collect();
    sorted.sort_by_key(|r| r.seq);
    if records.len() <= keep_recent {
        return sorted
            .iter()
            .map(|r| (r.seq, chat_blocks(&[(*r).clone()])))
            .collect();
    }
    let split = records.len().saturating_sub(keep_recent);
    let (old, recent) = sorted.split_at(split);
    let mut out: Vec<(u64, Vec<Block>)> = Vec::new();
    if let Some(last) = old.last() {
        out.push((last.seq, vec![ref_block(old)]));
    }
    out.extend(recent.iter().map(|r| (r.seq, chat_blocks(&[(*r).clone()]))));
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

fn ref_block(old: &[&ChatRecord]) -> Block {
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
