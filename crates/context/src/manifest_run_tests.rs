use proptest::prelude::*;

use crate::compile::compile;
use crate::fixtures::build_input;
use crate::input::BuildInput;
use crate::output::{BlockForm, BlockKind, Compiled, CompiledRequest, Manifest, ManifestEntry};
use crate::state::build_state;

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

fn forms_ok(request: &CompiledRequest, manifest: &Manifest) -> Option<String> {
    for b in &request.blocks {
        if matches!(
            b.kind,
            BlockKind::System
                | BlockKind::UserText
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
        if b.kind == BlockKind::UserRef && b.form != BlockForm::Reference {
            return Some("user ref block not marked reference".to_owned());
        }
    }
    for e in &manifest.included {
        if e.kind == BlockKind::UserRef && e.form != BlockForm::Reference {
            return Some("user ref manifest entry not marked reference".to_owned());
        }
    }
    None
}

fn has_entry(
    included: &[ManifestEntry],
    kind: BlockKind,
    pred: impl Fn(&ManifestEntry) -> bool,
) -> bool {
    included.iter().any(|e| e.kind == kind && pred(e))
}

fn must_included_ok(manifest: &Manifest, input: &BuildInput) -> Option<String> {
    let inc = &manifest.included;
    if !has_entry(inc, BlockKind::System, |e| e.form == BlockForm::Verbatim) {
        return Some("system missing from manifest".to_owned());
    }
    let state = build_state(&input.events);
    let counts_match = inc
        .iter()
        .filter(|e| e.kind == BlockKind::Constraint)
        .count()
        == input.constraints.len()
        && inc
            .iter()
            .filter(|e| e.kind == BlockKind::Criterion)
            .count()
            == state.criteria.len();
    if !counts_match {
        return Some("constraint or criterion count mismatch".to_owned());
    }
    let docs_ok = input
        .tool_specs
        .iter()
        .all(|s| has_entry(inc, BlockKind::ToolDoc, |e| e.source_ids.contains(&s.name)));
    let users_ok = input.user_turns.iter().all(|u| {
        has_entry(inc, BlockKind::UserText, |e| {
            e.source_ids.contains(&u.record_id) && e.form == BlockForm::Verbatim
        }) || has_entry(inc, BlockKind::UserRef, |e| {
            e.source_ids.contains(&u.record_id) && e.form == BlockForm::Reference
        })
    });
    if !(docs_ok && users_ok) {
        return Some("tool doc or user turn missing from manifest".to_owned());
    }
    tail_must_included_ok(inc, input, &state)
}

fn tail_must_included_ok(
    inc: &[ManifestEntry],
    input: &BuildInput,
    state: &crate::state::StateBuilder,
) -> Option<String> {
    let questions_ok = inc
        .iter()
        .filter(|e| e.kind == BlockKind::OpenQuestion)
        .count()
        >= input
            .open_questions
            .len()
            .saturating_add(state.open_cards().len())
        && state.open_cards().iter().all(|(seq, _)| {
            has_entry(inc, BlockKind::OpenQuestion, |e| {
                e.source_ids.contains(&format!("question-{seq}"))
            })
        });
    let current_ok = state
        .steps
        .iter()
        .filter(|(_, s)| s.state == crate::state::StepStateKind::Open)
        .max_by_key(|(_, s)| s.intent_seq)
        .is_none_or(|(id, _)| {
            has_entry(inc, BlockKind::CurrentStep, |e| e.source_ids.contains(id))
        });
    let pending_ok = state
        .pending
        .keys()
        .chain(state.unknowns.keys())
        .all(|id| has_entry(inc, BlockKind::PendingAction, |e| e.source_ids.contains(id)))
        && input.steps.iter().filter(|s| s.keeps_full()).all(|s| {
            has_entry(inc, BlockKind::PendingAction, |e| {
                e.source_ids.contains(&s.call_id.as_str())
            })
        });
    if !(questions_ok && current_ok && pending_ok) {
        return Some("tail context-must item missing from manifest".to_owned());
    }
    None
}

fn pinned_and_excluded_ok(
    request: &CompiledRequest,
    manifest: &Manifest,
    input: &BuildInput,
) -> Option<String> {
    let want = format!("pinned-v{}", input.pinned.version);
    if !has_entry(&manifest.included, BlockKind::Pinned, |e| {
        e.source_ids.iter().any(|id| id == &want)
    }) {
        return Some("pinned version missing from manifest".to_owned());
    }
    for e in &manifest.excluded {
        if e.source_id.is_empty() {
            return Some("excluded entry without source id".to_owned());
        }
    }
    for b in &request.blocks {
        let wanted_reason = if b.kind == BlockKind::ToolResultMasked {
            Some(crate::output::ExcludedReason::Masked)
        } else if b.kind == BlockKind::ToolResultDuplicate {
            Some(crate::output::ExcludedReason::Duplicate)
        } else {
            None
        };
        if let Some(reason) = wanted_reason {
            let has_match = manifest
                .excluded
                .iter()
                .any(|e| e.reason == reason && b.source_ids.contains(&e.source_id));
            if !has_match {
                return Some("excluded step missing reason in manifest".to_owned());
            }
        }
    }
    None
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_manifest_1(input in build_input(1_000, 200_000, 8)) {
        if let Compiled::Sent { request, manifest, .. } = compile(&input).unwrap() {
            prop_assert!(entries_match(&request, &manifest).is_none());
            prop_assert!(forms_ok(&request, &manifest).is_none());
            prop_assert!(must_included_ok(&manifest, &input).is_none());
            prop_assert!(pinned_and_excluded_ok(&request, &manifest, &input).is_none());
            let total: u64 = manifest.included.iter().map(|e| e.estimate_raw).sum();
            prop_assert_eq!(total, manifest.budget.estimate_raw);
            let cal = anybot_models::apply_calibration(total, &input.model.calibration);
            prop_assert_eq!(cal, manifest.budget.estimate_calibrated);
        }
    }
}
