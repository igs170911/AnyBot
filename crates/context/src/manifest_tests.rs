use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::output::{Compiled, ExcludedReason};

#[test]
fn manifest_json_omits_secret_shaped_text() {
    let mut base = sup::base_input();
    base.model = sup::plan(50_000);
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
fn manifest_can_store_reported_usage_and_delta() {
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
