use crate::build_tests::{rebuilt, step_at};
use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::StepRecord;
use crate::output::{Block, BlockKind, Compiled, ExcludedReason};

#[test]
fn rebuild_is_deterministic_with_forty_steps() {
    let mut base = sup::base_input();
    base.steps = (0..40usize)
        .map(|i| sup::step_len(i, 300, &format!("r{i}")))
        .collect();
    match (compile(&base).unwrap(), compile(&base).unwrap()) {
        (
            Compiled::Sent {
                request: a,
                manifest: ma,
                events: ea,
            },
            Compiled::Sent {
                request: b,
                manifest: mb,
                events: eb,
            },
        ) => {
            assert_eq!(a, b);
            assert_eq!(ma, mb);
            assert_eq!(ea, eb);
        }
        _ => panic!("both builds must be sent"),
    }
}

#[test]
fn watermark_ladder_drops_in_order() {
    let base = crate::compile_tests_support::base_input();
    let steps: Vec<StepRecord> = (0..14)
        .map(|i| step_at(i, 9_000, &format!("r{i}")))
        .collect();
    let ladder: Vec<u64> = vec![400_000, 150_000, 80_000, 40_000, 20_000, 9_000];
    let mut saw_masked = false;
    let mut saw_prior_ref = false;
    let mut saw_memory_drop = false;
    for usable in ladder {
        let input = rebuilt(&base, steps.clone(), Vec::new(), "v1");
        let mut input = input;
        input.model = crate::compile_tests_support::plan(usable);
        input.constraints = vec!["must keep me".to_owned()];
        match compile(&input).unwrap() {
            Compiled::Sent {
                request, manifest, ..
            } => {
                assert!(
                    request
                        .blocks
                        .iter()
                        .any(|b| b.kind == BlockKind::Constraint && b.text == "must keep me")
                );
                saw_masked = saw_masked
                    || request
                        .blocks
                        .iter()
                        .any(|b| b.kind == BlockKind::ToolResultMasked);
                saw_prior_ref = saw_prior_ref
                    || request
                        .blocks
                        .iter()
                        .any(|b| b.kind == BlockKind::PriorChatRef);
                saw_memory_drop = saw_memory_drop
                    || manifest.excluded.iter().any(|e| {
                        e.reason == ExcludedReason::Budget && e.source_id.starts_with('m')
                    });
                if saw_memory_drop {
                    assert!(saw_prior_ref, "prior refs drop before memories");
                }
                if saw_prior_ref {
                    assert!(saw_masked, "old results mask before prior refs drop");
                }
                let fulls: Vec<&Block> = request
                    .blocks
                    .iter()
                    .filter(|b| b.kind == BlockKind::ToolResultFull)
                    .collect();
                assert!(fulls.len() >= 2, "last two step results stay verbatim");
            }
            Compiled::Insufficient(x) => {
                assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }
}

#[test]
fn budget_ladder_constraints_survive_insufficient() {
    let base = crate::compile_tests_support::base_input();
    let steps: Vec<StepRecord> = (0..6)
        .map(|i| step_at(i, 40_000, &format!("r{i}")))
        .collect();
    let input = rebuilt(&base, steps, Vec::new(), "v1");
    let mut input = input;
    input.model = crate::compile_tests_support::plan(1_200);
    input.constraints = vec!["must keep me".to_owned(); 30];
    match compile(&input).unwrap() {
        Compiled::Sent { request, .. } => {
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::Constraint)
            );
        }
        Compiled::Insufficient(x) => {
            assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            assert!(x.deficit_tokens > 0);
        }
    }
}
