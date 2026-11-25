use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::{
    chat_record, events_strategy, mixed_text, model_plan, step_record, unique_step_refs,
};
use crate::input::{BuildInput, ChatRecord, StepRecord};
use crate::mask::render_view;
use crate::numbers::{K114_2_DEFAULT_RESULT_CHARS, K114_3_SHORT_RESULT_CHARS};
use crate::output::{Block, BlockKind, Compiled, CompiledRequest, Manifest};

fn has_block(request: &CompiledRequest, kind: BlockKind, pred: impl Fn(&Block) -> bool) -> bool {
    request.blocks.iter().any(|b| b.kind == kind && pred(b))
}

fn mask_2_problems(
    request: &CompiledRequest,
    users: &[ChatRecord],
    bots: &[ChatRecord],
    steps: &[StepRecord],
) -> Vec<String> {
    let mut problems = Vec::new();
    for u in users {
        if !has_block(request, BlockKind::UserText, |b| b.text == u.text) {
            problems.push(format!("user text missing {}", u.record_id));
        }
    }
    for bt in bots {
        if !has_block(request, BlockKind::BotText, |b| b.text == bt.text) {
            problems.push(format!("bot text missing {}", bt.record_id));
        }
    }
    for s in steps {
        let rid = format!("result-{}", s.source_ref);
        let cid = format!("call-{}", s.call_id.as_str());
        if s.has_output()
            && !request.blocks.iter().any(|x| {
                (x.kind == BlockKind::ToolResultFull
                    || x.kind == BlockKind::ToolResultMasked
                    || x.kind == BlockKind::ToolResultDuplicate)
                    && x.source_ids.contains(&rid)
            })
        {
            problems.push(format!("result block missing {rid}"));
        }
        if !has_block(request, BlockKind::ToolIntent, |x| {
            x.source_ids.contains(&cid)
        }) {
            problems.push(format!("intent block missing {cid}"));
        }
        if (s.keeps_full() || !s.has_output())
            && has_block(request, BlockKind::ToolResultMasked, |x| {
                x.source_ids.contains(&rid)
            })
        {
            problems.push(format!("unexpected mask for {rid}"));
        }
    }
    recent_two_problems(request, steps, &mut problems);
    problems
}

fn recent_two_problems(
    request: &CompiledRequest,
    steps: &[StepRecord],
    problems: &mut Vec<String>,
) {
    for s in steps.iter().rev().take(2) {
        if !s.has_output() {
            continue;
        }
        let rid = format!("result-{}", s.source_ref);
        let view = render_view(s, K114_2_DEFAULT_RESULT_CHARS);
        let seen = request.blocks.iter().any(|b| {
            (b.kind == BlockKind::ToolResultFull && b.text == view)
                || (b.kind == BlockKind::ToolResultDuplicate && b.source_ids.contains(&rid))
        });
        if !seen {
            problems.push(format!("recent two verbatim missing {rid}"));
        }
    }
}

struct LadderState {
    saw_masked: bool,
    saw_prior_ref: bool,
    saw_memory_drop: bool,
    prior_baseline: usize,
    first_round: bool,
}

fn ladder_input(steps: &[StepRecord], constraints: &[String], usable: u64) -> BuildInput {
    BuildInput {
        view_limit: K114_2_DEFAULT_RESULT_CHARS,
        events: Vec::new(),
        user_turns: Vec::new(),
        bot_turns: Vec::new(),
        steps: steps.to_vec(),
        constraints: constraints.to_vec(),
        open_questions: Vec::new(),
        tool_specs: Vec::new(),
        system_prompt: "sys".to_owned(),
        pinned: crate::input::PinnedSnapshot {
            version: "v".to_owned(),
            items: Vec::new(),
        },
        memories: (0..6)
            .map(|i| sup::memory(&format!("m{i}"), 50, false))
            .collect(),
        prior_chat: (0..12)
            .map(|i| sup::record(&format!("p{i}"), "old chat line"))
            .collect(),
        model: sup::plan(usable),
        compact_requested: false,
        secrets: Vec::new(),
    }
}

fn budget_excluded(manifest: &Manifest, prefix: char) -> usize {
    manifest
        .excluded
        .iter()
        .filter(|e| {
            e.reason == crate::output::ExcludedReason::Budget && e.source_id.starts_with(prefix)
        })
        .count()
}

fn ladder_round_problems(
    usable: u64,
    request: &CompiledRequest,
    manifest: &Manifest,
    steps: &[StepRecord],
    constraints: &[String],
    st: &mut LadderState,
) -> Vec<String> {
    let mut problems = Vec::new();
    for c in constraints {
        if !has_block(request, BlockKind::Constraint, |b| b.text == *c) {
            problems.push(format!("constraint missing {c}"));
        }
    }
    st.saw_masked = st.saw_masked || has_block(request, BlockKind::ToolResultMasked, |_| true);
    let prior_drops = budget_excluded(manifest, 'p');
    if st.first_round {
        st.prior_baseline = prior_drops;
        st.first_round = false;
    }
    st.saw_prior_ref = st.saw_prior_ref || prior_drops > st.prior_baseline;
    st.saw_memory_drop = st.saw_memory_drop || budget_excluded(manifest, 'm') > 0;
    if st.saw_memory_drop && !st.saw_prior_ref {
        problems.push(format!("memory dropped before old chat usable={usable}"));
    }
    if st.saw_prior_ref && !st.saw_masked && ladder_maskable(steps) > 2 {
        problems.push(format!("old results drop after old chat usable={usable}"));
    }
    recent_two_problems(request, steps, &mut problems);
    let fulls: usize = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::ToolResultFull || b.kind == BlockKind::ToolResultDuplicate)
        .count();
    if fulls > 0 && !steps.iter().any(StepRecord::has_output) {
        problems.push("result blocks without outputs".to_owned());
    }
    problems
}

fn ladder_maskable(steps: &[StepRecord]) -> usize {
    steps
        .iter()
        .filter(|s| {
            s.has_output()
                && !s.unconfirmed
                && s.content.chars().count() > K114_3_SHORT_RESULT_CHARS
        })
        .count()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_mask_2(
        raw_steps in prop::collection::vec(step_record(0), 1..10),
        users in prop::collection::vec(chat_record("u", 0), 1..3),
        bots in prop::collection::vec(chat_record("b", 0), 0..3),
        events in events_strategy(6),
        model in model_plan(60_000, 300_000),
    ) {
        let steps = unique_step_refs(raw_steps);
        let input = BuildInput {
            view_limit: K114_2_DEFAULT_RESULT_CHARS,
            events,
            user_turns: users.clone(),
            bot_turns: bots.clone(),
            steps: steps.clone(),
            constraints: Vec::new(),
            open_questions: Vec::new(),
            tool_specs: Vec::new(),
            system_prompt: "sys".to_owned(),
            pinned: crate::input::PinnedSnapshot { version: "v".to_owned(), items: Vec::new() },
            memories: Vec::new(),
            prior_chat: Vec::new(),
            model,
            compact_requested: false,
            secrets: Vec::new(),
        };
        match compile(&input) {
            Compiled::Sent { request, .. } => {
                let problems = mask_2_problems(&request, &users, &bots, &steps);
                prop_assert!(problems.is_empty(), "{problems:?}");
            }
            Compiled::Insufficient(x) => {
                prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }

    #[test]
    fn t_context_budget_2(
        raw_steps in prop::collection::vec(step_record(0), 3..12),
        constraints in prop::collection::vec(mixed_text(30), 1..3),
    ) {
        let steps = unique_step_refs(raw_steps);
        let mut st = LadderState {
            saw_masked: false,
            saw_prior_ref: false,
            saw_memory_drop: false,
            prior_baseline: 0,
            first_round: true,
        };
        for usable in [200_000u64, 60_000, 20_000, 8_000] {
            let input = ladder_input(&steps, &constraints, usable);
            match compile(&input) {
                Compiled::Sent { request, manifest, .. } => {
                    let problems =
                        ladder_round_problems(usable, &request, &manifest, &steps, &constraints, &mut st);
                    prop_assert!(problems.is_empty(), "{problems:?}");
                }
                Compiled::Insufficient(x) => {
                    prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
                }
            }
        }
    }
}
