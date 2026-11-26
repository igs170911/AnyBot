use proptest::prelude::*;

use crate::compile::compile;
use crate::fixtures::build_input;
use crate::input::{BuildInput, ChatRecord, ModelPlan, StepRecord};
use crate::numbers::{K114_4_WATERMARK_MILLI, milli_of};
use crate::output::{BlockKind, Compiled, ExcludedReason, Manifest, NewEvent};

fn cheap_input(low: u64, high: u64, max_steps: usize) -> BoxedStrategy<BuildInput> {
    build_input(low, high, max_steps)
        .prop_map(|mut b| {
            let steps: Vec<StepRecord> = b
                .steps
                .iter()
                .map(|s| StepRecord {
                    content: s.content.chars().take(1_200).collect(),
                    ..s.clone()
                })
                .collect();
            let trim = |v: Vec<ChatRecord>| -> Vec<ChatRecord> {
                v.iter()
                    .map(|r| ChatRecord {
                        text: r.text.chars().take(300).collect(),
                        ..r.clone()
                    })
                    .collect()
            };
            b.steps = steps;
            b.user_turns = trim(b.user_turns);
            b.bot_turns = trim(b.bot_turns);
            b.prior_chat = trim(b.prior_chat);
            b.constraints = b
                .constraints
                .iter()
                .map(|c| c.chars().take(300).collect())
                .collect();
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

fn full_content(request: &crate::output::CompiledRequest, step: &StepRecord) -> bool {
    request.blocks.iter().any(|b| {
        b.kind == BlockKind::ToolResultFull
            && b.text == step.content
            && b.call_id.as_deref() == Some(step.call_id.as_str().as_str())
    })
}

fn must_survives(input: &BuildInput, request: &crate::output::CompiledRequest) -> bool {
    let constraints_ok = input.constraints.iter().all(|c| {
        request
            .blocks
            .iter()
            .any(|b| b.kind == BlockKind::Constraint && b.text == *c)
    });
    let users_ok = input.user_turns.iter().all(|u| {
        request.blocks.iter().any(|b| {
            (b.kind == BlockKind::UserText && b.text == u.text)
                || (b.kind == BlockKind::UserRef && b.text.contains(&u.record_id))
        })
    });
    let system_ok = request
        .blocks
        .iter()
        .any(|b| b.kind == BlockKind::System && b.text == input.system_prompt);
    let docs_ok = input.tool_specs.iter().all(|s| {
        request
            .blocks
            .iter()
            .any(|b| b.kind == BlockKind::ToolDoc && b.text.contains(&s.name))
    });
    constraints_ok && users_ok && system_ok && docs_ok
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
        } else if e.source_id.starts_with('m') {
            d.memories = true;
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

fn recent_two_missing(input: &BuildInput, request: &crate::output::CompiledRequest) -> bool {
    let n = input.steps.len();
    for idx in [n.saturating_sub(2), n.saturating_sub(1)] {
        let Some(step) = input.steps.get(idx) else {
            continue;
        };
        if !step.has_output() {
            continue;
        }
        if step.content.chars().count() > input.view_limit {
            continue;
        }
        if !full_content(request, step) {
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

fn order_broken(d: &Dropped) -> bool {
    d.prior && !d.steps || !d.prior && d.memories
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_budget_2(
        base in cheap_input(58_000, 62_000, 8),
        shrinks in prop::collection::vec(5_000usize..55_000usize, 3..6),
    ) {
        let mut base = base;
        base.steps = crate::fixtures::unique_step_refs(base.steps);
        for shrink in shrinks {
            let usable = 60_000u64.saturating_sub(u64::try_from(shrink).unwrap_or(0));
            let input = shrink_build(&base, usable);
            match compile(&input).unwrap() {
                Compiled::Sent { request, manifest, events, .. } => {
                    prop_assert!(must_survives(&input, &request), "context-must items missing");
                    prop_assert!(
                        !recent_two_missing(&input, &request),
                        "recent step content missing"
                    );
                    let d = dropped(&manifest, &events);
                    prop_assert!(!order_broken(&d), "S2 trim order broken");
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
