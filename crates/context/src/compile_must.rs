use crate::assemble::block;
use crate::compile::Assembled;
use crate::input::{BuildInput, StepRecord};
use crate::mask::result_summary;
use crate::output::{Block, BlockForm, BlockKind};

pub(crate) fn mandatory_raw(a: &Assembled) -> u64 {
    a.core
        .iter()
        .filter(|b| is_mandatory(b.kind))
        .map(|b| b.estimate_raw)
        .sum()
}

pub(crate) fn is_mandatory(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::UserText
            | BlockKind::UserRef
            | BlockKind::Constraint
            | BlockKind::Criterion
            | BlockKind::CurrentStep
            | BlockKind::OpenQuestion
            | BlockKind::PendingAction
            | BlockKind::ToolDoc
    )
}

pub(crate) fn must_blocks(input: &BuildInput, state: &crate::state::StateBuilder) -> Vec<Block> {
    let mut out = Vec::new();
    for (idx, c) in state.criteria.iter().enumerate() {
        out.push(block(
            BlockKind::Criterion,
            BlockForm::Verbatim,
            vec![format!("criterion-{idx}")],
            c.text.clone(),
        ));
    }
    if let Some(step) = open_step(state) {
        out.push(block(
            BlockKind::CurrentStep,
            BlockForm::Verbatim,
            vec![step.0.clone()],
            format!("current step: {} [seq {}]", step.1.tool, step.1.intent_seq),
        ));
    }
    for (seq, action) in &state.questions {
        out.push(block(
            BlockKind::OpenQuestion,
            BlockForm::Verbatim,
            vec![format!("question-{seq}")],
            format!("open question: {action}"),
        ));
    }
    for (id, seq) in state.pending.iter().chain(state.unknowns.iter()) {
        out.push(block(
            BlockKind::PendingAction,
            BlockForm::Verbatim,
            vec![id.clone()],
            format!("action {id} [seq {seq}] needs confirmation"),
        ));
    }
    out.extend(
        input
            .steps
            .iter()
            .filter(|s| s.keeps_full())
            .map(pending_step_block),
    );
    out
}

fn open_step(state: &crate::state::StateBuilder) -> Option<(&String, &crate::state::StepState)> {
    state
        .steps
        .iter()
        .filter(|(_, s)| s.state == crate::state::StepStateKind::Open)
        .max_by_key(|(_, s)| s.intent_seq)
}

fn pending_step_block(step: &StepRecord) -> Block {
    block(
        BlockKind::PendingAction,
        BlockForm::Verbatim,
        vec![step.call_id.as_str().clone()],
        format!(
            "pending step {} {} [{}]",
            step.call_id.as_str(),
            step.tool,
            result_summary(step)
        ),
    )
}

pub(crate) fn block_summary(summary: &crate::state_render::StateSummary) -> Block {
    block(
        BlockKind::StateSummary,
        BlockForm::Summary,
        Vec::new(),
        summary.text(),
    )
}
