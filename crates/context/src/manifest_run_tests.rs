use proptest::prelude::*;

use crate::compile::compile;
use crate::fixtures::build_input;
use crate::input::BuildInput;
use crate::output::{BlockForm, BlockKind, Compiled, CompiledRequest, Manifest};

fn entries_match(request: &CompiledRequest, manifest: &Manifest) -> Option<String> {
    if manifest.included.len() != request.blocks.len() {
        return Some(format!(
            "included {} != blocks {}",
            manifest.included.len(),
            request.blocks.len()
        ));
    }
    for (entry, block) in manifest.included.iter().zip(request.blocks.iter()) {
        if entry.estimate_raw != block.estimate_raw
            || entry.kind != block.kind
            || entry.form != block.form
            || entry.source_ids != block.source_ids
        {
            return Some(format!("entry {entry:?} != block {block:?}"));
        }
    }
    None
}

fn verbatim_ok(request: &CompiledRequest) -> Option<String> {
    for b in &request.blocks {
        if matches!(
            b.kind,
            BlockKind::UserText
                | BlockKind::Constraint
                | BlockKind::Criterion
                | BlockKind::ToolDoc
                | BlockKind::Pinned
                | BlockKind::CurrentStep
                | BlockKind::OpenQuestion
                | BlockKind::PendingAction
        ) && b.form != BlockForm::Verbatim
        {
            return Some("mandatory block not verbatim".to_owned());
        }
    }
    None
}

fn counts_ok(request: &CompiledRequest, input: &BuildInput) -> Option<String> {
    let count = |k: BlockKind| request.blocks.iter().filter(|b| b.kind == k).count();
    if count(BlockKind::Constraint) != input.constraints.len() {
        return Some("constraint count mismatch".to_owned());
    }
    if count(BlockKind::ToolDoc) != input.tool_specs.len() {
        return Some("tool doc count mismatch".to_owned());
    }
    let users = request
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::UserText | BlockKind::UserRef))
        .count();
    if users != input.user_turns.len() {
        return Some("user turn count mismatch".to_owned());
    }
    None
}

fn pinned_ok(request: &CompiledRequest, input: &BuildInput) -> Option<String> {
    let want = format!("pinned-v{}", input.pinned.version);
    let found = request
        .blocks
        .iter()
        .any(|b| b.kind == BlockKind::Pinned && b.source_ids.iter().any(|id| id == &want));
    if !found {
        return Some("pinned version block missing".to_owned());
    }
    None
}

fn excluded_ok(manifest: &Manifest) -> Option<String> {
    for e in &manifest.excluded {
        if e.source_id.is_empty() {
            return Some("excluded entry without source id".to_owned());
        }
    }
    None
}

fn system_ok(request: &CompiledRequest, manifest: &Manifest) -> Option<String> {
    let has_system = request.blocks.iter().any(|b| b.kind == BlockKind::System);
    if !has_system {
        return Some("system block missing from request".to_owned());
    }
    if !manifest
        .included
        .iter()
        .any(|e| e.kind == BlockKind::System)
    {
        return Some("system block missing from manifest".to_owned());
    }
    None
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn manifest_entries_match_request_blocks(input in build_input(1_000, 200_000, 8)) {
        if let Compiled::Sent { request, manifest, .. } = compile(&input).unwrap() {
            prop_assert!(entries_match(&request, &manifest).is_none());
            prop_assert!(verbatim_ok(&request).is_none());
            prop_assert!(counts_ok(&request, &input).is_none());
            prop_assert!(pinned_ok(&request, &input).is_none());
            let total: u64 = manifest.included.iter().map(|e| e.estimate_raw).sum();
            prop_assert_eq!(total, manifest.budget.estimate_raw);
            prop_assert!(excluded_ok(&manifest).is_none());
            prop_assert!(system_ok(&request, &manifest).is_none());
        }
    }
}
