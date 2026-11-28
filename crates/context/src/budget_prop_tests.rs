use proptest::prelude::*;

use crate::compile::compile;
use crate::fixtures::{build_input, unique_memory_ids, unique_step_refs};
use crate::input::{BuildInput, ChatRecord, ModelPlan, StepRecord};
use crate::mask::render_view;
use crate::numbers::{K114_3_SHORT_RESULT_CHARS, K114_4_WATERMARK_MILLI, milli_of};
use crate::output::{
    Block, BlockKind, Compiled, CompiledRequest, ExcludedReason, Manifest, NewEvent,
};
use crate::state::build_state;

fn budget_input(low: u64, high: u64, max_steps: usize) -> BoxedStrategy<BuildInput> {
    (
        build_input(low, high, max_steps),
        prop::collection::vec(0u8..2u8, 4..10),
        prop::collection::vec(0u8..2u8, 4..12),
    )
        .prop_map(|(mut b, mem_pad, prior_pad)| {
            b.steps = unique_step_refs(
                b.steps
                    .into_iter()
                    .map(|s| StepRecord {
                        content: s.content.chars().take(1_200).collect(),
                        ..s
                    })
                    .collect(),
            );
            if b.memories.len() < mem_pad.len() {
                let extra = mem_pad.len().saturating_sub(b.memories.len());
                for idx in 0..extra {
                    b.memories.push(crate::input::MemoryRecord {
                        id: format!("mx{idx}"),
                        content: format!("memory body {idx} {}", "m".repeat(220)),
                        relevance: u32::try_from(idx.saturating_mul(10)).unwrap_or(50),
                        source: crate::input::MemorySource::UserSaid,
                        candidate: false,
                    });
                }
            }
            b.memories = unique_memory_ids(b.memories);
            if b.prior_chat.len() < prior_pad.len() {
                let start = b.prior_chat.len();
                for idx in start..prior_pad.len() {
                    b.prior_chat.push(ChatRecord {
                        record_id: format!("px-{idx}"),
                        text: format!("prior chat {idx} {}", "p".repeat(360)),
                        seq: u64::try_from(idx).unwrap_or(0).saturating_add(1),
                    });
                }
            }
            b.prior_chat = crate::fixtures::renumbered(b.prior_chat);
            b
        })
        .boxed()
}

fn shrink_model(input: &BuildInput, usable: u64) -> ModelPlan {
    let mut model = input.model.clone();
    model.capability.context_tokens = usable.saturating_add(input.model.reserved_output);
    model
}

fn shrink_build(base: &BuildInput, usable: u64) -> BuildInput {
    BuildInput {
        view_limit: base.view_limit,
        events: base.events.clone(),
        user_turns: base.user_turns.clone(),
        bot_turns: base.bot_turns.clone(),
        steps: base.steps.clone(),
        constraints: base.constraints.clone(),
        open_questions: base.open_questions.clone(),
        tool_specs: base.tool_specs.clone(),
        system_prompt: base.system_prompt.clone(),
        pinned: base.pinned.clone(),
        memories: base.memories.clone(),
        prior_chat: base.prior_chat.clone(),
        model: shrink_model(base, usable),
        compact_requested: base.compact_requested,
    }
}

fn has_block(request: &CompiledRequest, kind: BlockKind, pred: impl Fn(&Block) -> bool) -> bool {
    request.blocks.iter().any(|b| b.kind == kind && pred(b))
}

fn must_survives(input: &BuildInput, request: &CompiledRequest) -> bool {
    let state = build_state(&input.events);
    let constraints_ok = input
        .constraints
        .iter()
        .all(|c| has_block(request, BlockKind::Constraint, |b| b.text == *c));
    let criteria_ok = state
        .criteria
        .iter()
        .all(|c| has_block(request, BlockKind::Criterion, |b| b.text == c.text));
    let users_ok = input.user_turns.iter().all(|u| {
        has_block(request, BlockKind::UserText, |b| b.text == u.text)
            || has_block(request, BlockKind::UserRef, |b| {
                b.source_ids.contains(&u.record_id)
            })
    });
    let questions_ok = input
        .open_questions
        .iter()
        .all(|q| has_block(request, BlockKind::OpenQuestion, |b| b.text == *q))
        && state.open_cards().iter().all(|(seq, _)| {
            has_block(request, BlockKind::OpenQuestion, |b| {
                b.source_ids.contains(&format!("question-{seq}"))
            })
        });
    let current_ok = state
        .steps
        .iter()
        .filter(|(_, s)| s.state == crate::state::StepStateKind::Open)
        .max_by_key(|(_, s)| s.intent_seq)
        .is_none_or(|(id, _)| {
            has_block(request, BlockKind::CurrentStep, |b| {
                b.source_ids.contains(id)
            })
        });
    let pending_ok = state.pending.keys().chain(state.unknowns.keys()).all(|id| {
        has_block(request, BlockKind::PendingAction, |b| {
            b.source_ids.contains(id)
        })
    }) && input.steps.iter().filter(|s| s.keeps_full()).all(|s| {
        has_block(request, BlockKind::PendingAction, |b| {
            b.source_ids.contains(&s.call_id.as_str())
        })
    });
    let system_ok = has_block(request, BlockKind::System, |b| {
        b.text == input.system_prompt
    });
    let docs_ok = input
        .tool_specs
        .iter()
        .all(|s| has_block(request, BlockKind::ToolDoc, |b| b.text.contains(&s.name)));
    constraints_ok
        && criteria_ok
        && users_ok
        && questions_ok
        && current_ok
        && pending_ok
        && system_ok
        && docs_ok
}

struct Dropped {
    steps: bool,
    prior: bool,
    memories: bool,
}

fn dropped(manifest: &Manifest, events: &[NewEvent]) -> Dropped {
    let mut d = Dropped {
        steps: false,
        prior: false,
        memories: false,
    };
    for e in &manifest.excluded {
        if e.reason != ExcludedReason::Budget {
            continue;
        }
        if e.source_id.starts_with("ref") {
            d.steps = true;
        } else if e.source_id.starts_with('p') {
            d.prior = true;
        }
    }
    for e in events {
        match e {
            NewEvent::Collapsed { ids, reason } => {
                if *reason == crate::output::CollapseReason::Budget {
                    if ids.iter().any(|i| i.starts_with("ref")) {
                        d.steps = true;
                    }
                    if ids.iter().any(|i| i.starts_with('p')) {
                        d.prior = true;
                    }
                }
            }
            NewEvent::MemoryDropped { .. } => d.memories = true,
            NewEvent::ContextBuilt => {}
        }
    }
    d
}

fn recent_two_missing(input: &BuildInput, request: &CompiledRequest) -> bool {
    let n = input.steps.len();
    for idx in [n.saturating_sub(2), n.saturating_sub(1)] {
        let Some(step) = input.steps.get(idx) else {
            continue;
        };
        if !step.has_output() {
            continue;
        }
        let view = render_view(step, input.view_limit);
        let rid = format!("result-{}", step.source_ref);
        let seen = request.blocks.iter().any(|b| {
            (b.kind == BlockKind::ToolResultFull && b.text == view)
                || (b.kind == BlockKind::ToolResultDuplicate && b.source_ids.contains(&rid))
        });
        if !seen {
            return true;
        }
    }
    false
}

fn trim_rank(event: &NewEvent) -> Option<usize> {
    match event {
        NewEvent::Collapsed { ids, reason } => {
            if *reason != crate::output::CollapseReason::Budget {
                return None;
            }
            if ids.iter().any(|i| i.starts_with("ref")) {
                Some(0)
            } else if ids.iter().any(|i| i.starts_with('p')) {
                Some(1)
            } else {
                None
            }
        }
        NewEvent::MemoryDropped { .. } => Some(2),
        NewEvent::ContextBuilt => None,
    }
}

fn trim_order_ok(events: &[NewEvent]) -> bool {
    let mut prev = 0usize;
    for e in events {
        if let Some(r) = trim_rank(e) {
            if r < prev {
                return false;
            }
            prev = r;
        }
    }
    true
}

fn order_broken(input: &BuildInput, d: &Dropped) -> bool {
    let skipped_results = d.prior && !d.steps && droppable_results(input);
    let skipped_prior = d.memories && !d.prior && !input.prior_chat.is_empty();
    skipped_results || skipped_prior
}

fn droppable_results(input: &BuildInput) -> bool {
    let cutoff = input.steps.len().saturating_sub(2);
    input.steps.iter().take(cutoff).any(|step| {
        !step.keeps_full()
            && step.has_output()
            && step.content.chars().count() > K114_3_SHORT_RESULT_CHARS
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_budget_2(
        base in budget_input(58_000, 62_000, 10),
        raw_shrinks in prop::collection::vec(10_000u64..58_500u64, 4..7),
    ) {
        let mut usables: Vec<u64> = raw_shrinks
            .into_iter()
            .map(|s| 60_000u64.saturating_sub(s))
            .collect();
        usables.sort_unstable_by(|a, b| b.cmp(a));
        for usable in usables {
            let input = shrink_build(&base, usable);
            match compile(&input).unwrap() {
                Compiled::Sent { request, manifest, events, .. } => {
                    prop_assert!(must_survives(&input, &request), "context-must items missing");
                    prop_assert!(
                        !recent_two_missing(&input, &request),
                        "recent step content missing"
                    );
                    let d = dropped(&manifest, &events);
                    prop_assert!(!order_broken(&input, &d), "S2 trim order broken");
                    prop_assert!(trim_order_ok(&events), "S2 trim event order broken");
                    let cal = manifest.budget.estimate_calibrated;
                    prop_assert!(
                        cal <= milli_of(usable, K114_4_WATERMARK_MILLI) || cal <= usable,
                        "estimate {cal} over usable {usable}"
                    );
                }
                Compiled::Insufficient(x) => {
                    prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
                }
            }
        }
    }
}
