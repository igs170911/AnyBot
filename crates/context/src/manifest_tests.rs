use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::build_input;
use crate::output::{BlockKind, Compiled, ExcludedReason};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn manifest_matches_request(input in build_input(1_000, 200_000, 8)) {
        if let Compiled::Sent { request, manifest, .. } = compile(&input).unwrap() {
            prop_assert_eq!(manifest.included.len(), request.blocks.len());
            for (entry, block) in manifest.included.iter().zip(request.blocks.iter()) {
                prop_assert_eq!(entry.estimate_raw, block.estimate_raw);
                prop_assert_eq!(&entry.kind, &block.kind);
                prop_assert_eq!(&entry.form, &block.form);
            }
            let total: u64 = manifest.included.iter().map(|e| e.estimate_raw).sum();
            prop_assert_eq!(total, manifest.budget.estimate_raw);
            for e in &manifest.excluded {
                prop_assert!(!e.source_id.is_empty());
            }
            for b in &request.blocks {
                if b.kind == BlockKind::System {
                    prop_assert!(manifest.included.iter().any(|e| e.kind == BlockKind::System));
                }
            }
            prop_assert!(request.blocks.iter().any(|b| b.kind == BlockKind::System));
        }
    }
}

#[test]
fn manifest_excludes_raw_records_and_secrets() {
    let mut base = sup::base_input();
    base.model = sup::plan(50_000);
    let hidden = sup::secret("hunter2");
    base.secrets = vec![hidden];
    base.memories = vec![sup::memory("m0", 90, false)];
    base.steps = vec![sup::step_len(0, 800, "r0")];
    match compile(&base).unwrap() {
        Compiled::Sent { manifest, .. } => {
            let text = serde_json::to_string(&manifest).unwrap();
            assert!(!text.contains("hunter2"));
            assert!(
                manifest
                    .included
                    .iter()
                    .all(|e| e.source_ids.iter().all(|id| !id.contains("hunter2")))
            );
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn manifest_usage_matches_response_stream() {
    let mut base = sup::base_input();
    base.steps = vec![sup::step_len(0, 300, "r0")];
    match compile(&base).unwrap() {
        Compiled::Sent { manifest, .. } => {
            assert!(manifest.actual_input_tokens.is_none());
            assert!(manifest.estimate_delta.is_none());
            let mut filled = manifest.clone();
            filled.actual_input_tokens = Some(1_234);
            let delta = i64::try_from(1_234u64)
                .unwrap_or(0)
                .saturating_sub(i64::try_from(filled.budget.estimate_calibrated).unwrap_or(0));
            filled.estimate_delta = Some(delta);
            assert_eq!(filled.actual_input_tokens, Some(1_234));
            assert!(filled.estimate_delta.is_some());
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn excluded_budget_reasons_recorded() {
    let mut base = sup::base_input();
    base.model = sup::plan(3_000);
    base.memories = (0..8)
        .map(|i| sup::memory(&format!("m{i}"), 10, false))
        .collect();
    base.prior_chat = (0..40)
        .map(|i| sup::record(&format!("p{i}"), &"q".repeat(400)))
        .collect();
    match compile(&base).unwrap() {
        Compiled::Sent { manifest, .. } => {
            assert!(
                manifest
                    .excluded
                    .iter()
                    .any(|e| e.reason == ExcludedReason::Budget)
            );
        }
        Compiled::Insufficient(_) => {}
    }
}
