use proptest::prelude::*;

use crate::compile::compile;
use crate::fixtures::{build_input, event_with_seq, events_strategy, mixed_text};
use crate::input::{BuildInput, PinnedSnapshot, StepRecord};
use crate::output::{Block, BlockKind, Compiled, CompiledRequest, NewEvent};

use crate::compile_tests_support as sup;

pub(crate) fn step_at(index: usize, len: usize, ref_id: &str) -> StepRecord {
    sup::step_len(index, len, ref_id)
}

pub(crate) fn rebuilt(
    base: &BuildInput,
    steps: Vec<StepRecord>,
    events: Vec<anybot_protocol::TaskEvent>,
    version: &str,
) -> BuildInput {
    BuildInput {
        view_limit: base.view_limit,
        events,
        user_turns: base.user_turns.clone(),
        bot_turns: base.bot_turns.clone(),
        steps,
        constraints: base.constraints.clone(),
        open_questions: base.open_questions.clone(),
        tool_specs: base.tool_specs.clone(),
        system_prompt: base.system_prompt.clone(),
        pinned: PinnedSnapshot {
            version: version.to_owned(),
            items: vec!["pin".to_owned()],
        },
        memories: base.memories.clone(),
        prior_chat: base.prior_chat.clone(),
        model: base.model.clone(),
        compact_requested: base.compact_requested,
        secrets: Vec::new(),
    }
}

pub(crate) fn rebuilt_constraints(base: &BuildInput, constraints: Vec<String>) -> BuildInput {
    let mut out = rebuilt(base, base.steps.clone(), base.events.clone(), "v1");
    out.constraints = constraints;
    out
}

pub(crate) fn prefix_of(request: &CompiledRequest) -> Vec<Block> {
    request
        .blocks
        .iter()
        .take(request.prefix_len)
        .cloned()
        .collect()
}

pub(crate) fn collapse_recorded(events: &[anybot_protocol::TaskEvent]) -> bool {
    events.iter().any(|e| {
        (anybot_domain::parse_collapsed(&e.kind).unwrap()).is_some_and(|c| !c.ids.is_empty())
    })
}

pub(crate) fn model_swap_present(events: &[anybot_protocol::TaskEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            &e.kind,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ModelSwapped { .. }
            )
        )
    })
}

pub(crate) fn push_built(events: &mut Vec<anybot_protocol::TaskEvent>) {
    let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
    events.push(event_with_seq(
        seq,
        anybot_protocol::TaskEventKind::Known(anybot_protocol::KnownTaskEventKind::ContextBuilt),
    ));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_build_1(input in build_input(1_000, 100_000, 8)) {
        let first = compile(&input).unwrap();
        let second = compile(&input).unwrap();
        let a = match &first {
            Compiled::Sent { request, .. } => serde_json::to_vec(request).unwrap(),
            Compiled::Insufficient(x) => serde_json::to_vec(x).unwrap(),
        };
        let b = match &second {
            Compiled::Sent { request, .. } => serde_json::to_vec(request).unwrap(),
            Compiled::Insufficient(x) => serde_json::to_vec(x).unwrap(),
        };
        prop_assert_eq!(a, b);
    }

    #[test]
    fn mandatory_verbatim_or_refuse(
        constraints in prop::collection::vec(mixed_text(40), 1..4),
        events in events_strategy(8),
        input in build_input(80_000, 400_000, 4),
    ) {
        let steps = vec![step_at(0, 200, "r0"), step_at(1, 300, "r1")];
        let base = rebuilt(&input, steps, events.clone(), "v1");
        let with_constraints = rebuilt_constraints(&base, constraints.clone());
        match compile(&with_constraints).unwrap() {
            Compiled::Sent { request, .. } => {
                for c in &constraints {
                    prop_assert!(request.blocks.iter().any(|b| b.kind == BlockKind::Constraint && b.text == *c));
                }
                for e in &events {
                    if let anybot_protocol::TaskEventKind::Known(
                        anybot_protocol::KnownTaskEventKind::DoneWhenChange { after, .. },
                    ) = &e.kind
                    {
                        for item in after {
                            let text = crate::state::criterion_text(item);
                            prop_assert!(request.blocks.iter().any(|b| b.kind == BlockKind::Criterion && b.text == text));
                        }
                    }
                }
                for u in &base.user_turns {
                    let ok = request.blocks.iter().any(|b| {
                        (b.kind == BlockKind::UserText && b.text == u.text)
                            || (b.kind == BlockKind::UserRef
                                && b.text.starts_with(&format!("ref {} len {}", u.record_id, u.text.chars().count())))
                    });
                    prop_assert!(ok, "user turn present {}", u.record_id);
                }
                prop_assert!(request.blocks.iter().any(|b| b.kind == BlockKind::System && b.text == base.system_prompt));
            }
            Compiled::Insufficient(x) => {
                prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }

    #[test]
    fn budget_never_oversends(input in build_input(500, 60_000, 6)) {
        match compile(&input).unwrap() {
            Compiled::Sent { manifest, request, .. } => {
                let usable = input.model.usable();
                prop_assert!(manifest.budget.estimate_calibrated <= usable);
                let raw: u64 = manifest.included.iter().map(|e| e.estimate_raw).sum();
                prop_assert_eq!(raw, manifest.budget.estimate_raw);
                prop_assert_eq!(manifest.included.len(), request.blocks.len());
            }
            Compiled::Insufficient(x) => {
                prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }
}

proptest! {
    #[test]
    fn prefix_stable_across_steps(steps in prop::collection::vec(0usize..3_000usize, 1..12), tick in 0..700u64) {
        let base = crate::compile_tests_support::base_input();
        let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
        let mut prev: Option<(usize, Vec<Block>)> = None;
        let mut version: u64 = 1;
        for (index, len) in steps.iter().enumerate() {
            if tick % 7 == 0 && index % 5 == 4 {
                version = version.saturating_add(1);
            }
            let many: Vec<StepRecord> = (0..=index).map(|i| step_at(i, *len, &format!("r{i}"))).collect();
            let input = rebuilt(&base, many, events.clone(), &version.to_string());
            let compiled = compile(&input).unwrap();
            if let Compiled::Sent { request, events: new, .. } = &compiled {
                for e in new {
                    match e {
                        NewEvent::ContextBuilt => push_built(&mut events),
                        NewEvent::MemoryDropped { ids } => {
                            let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
                            events.push(event_with_seq(
                                seq,
                                anybot_domain::memory_dropped_kind(ids),
                            ));
                        }
                        NewEvent::Collapsed { ids, reason } => {
                            let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
                            events.push(event_with_seq(
                                seq,
                                anybot_domain::collapsed_kind(&anybot_domain::ContextCollapsed { ids: ids.clone(), reason: reason_to_domain(*reason) }),
                            ));
                        }
                    }
                }
                let current = prefix_of(request);
                if let Some((prev_len, prev_prefix)) = &prev {
                    let keep = prev_prefix.len().min(current.len());
                    let same = prev_prefix.iter().take(keep).eq(current.iter().take(keep));
                    let grew = current.len() > *prev_len;
                    if !same {
                        prop_assert!(
                            collapse_recorded(&events)
                                || model_swap_present(&events)
                                || grew
                                || tick % 7 == 0
                        );
                    }
                }
                prev = Some((request.prefix_len, current));
            } else {
                push_built(&mut events);
            }
        }
    }
}

pub(crate) fn reason_to_domain(
    reason: crate::output::CollapseReason,
) -> anybot_domain::ContextCollapseReason {
    match reason {
        crate::output::CollapseReason::Accumulated => {
            anybot_domain::ContextCollapseReason::Accumulated
        }
        crate::output::CollapseReason::Budget => anybot_domain::ContextCollapseReason::Budget,
        crate::output::CollapseReason::Compact => anybot_domain::ContextCollapseReason::Compact,
        crate::output::CollapseReason::ModelSwapped => {
            anybot_domain::ContextCollapseReason::ModelSwapped
        }
        crate::output::CollapseReason::ReplaceAll => {
            anybot_domain::ContextCollapseReason::ReplaceAll
        }
    }
}
