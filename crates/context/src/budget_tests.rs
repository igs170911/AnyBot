use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::{BuildInput, PinnedSnapshot, StepRecord};
use crate::numbers::K114_1_RECENT_STEPS;
use crate::output::{BlockKind, Compiled, NewEvent};

pub(crate) fn assemble_input(
    steps: Vec<StepRecord>,
    usable: u64,
    events: Vec<anybot_protocol::TaskEvent>,
    prior_chat: Vec<crate::input::ChatRecord>,
) -> BuildInput {
    with_turns(steps, usable, events, prior_chat, Vec::new())
}

fn with_turns(
    steps: Vec<StepRecord>,
    usable: u64,
    events: Vec<anybot_protocol::TaskEvent>,
    prior_chat: Vec<crate::input::ChatRecord>,
    user_turns: Vec<crate::input::ChatRecord>,
) -> BuildInput {
    BuildInput {
        view_limit: crate::numbers::K114_2_DEFAULT_RESULT_CHARS,
        events,
        user_turns,
        bot_turns: Vec::new(),
        steps,
        constraints: Vec::new(),
        open_questions: Vec::new(),
        tool_specs: Vec::new(),
        system_prompt: "sys".to_owned(),
        pinned: PinnedSnapshot {
            version: "v1".to_owned(),
            items: Vec::new(),
        },
        memories: Vec::new(),
        prior_chat,
        model: sup::plan(usable),
        compact_requested: false,
    }
}

#[test]
fn smaller_model_collapses_more_then_larger_restores_recent() {
    let steps: Vec<StepRecord> = (0..30)
        .map(|i| sup::step_len(i, 1_500, &format!("r{i}")))
        .collect();
    let mut events: Vec<anybot_protocol::TaskEvent> = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    )];
    let big = budget_3_input(steps.clone(), 128_000, events.clone());
    match compile(&big).unwrap() {
        Compiled::Sent { events: new, .. } => sup::push_new(&mut events, &new),
        Compiled::Insufficient(_) => panic!("big model must fit"),
    }
    events.push(sup::known(
        u64::try_from(events.len()).unwrap_or(0).saturating_add(1),
        anybot_protocol::KnownTaskEventKind::ModelSwapped {
            from: "big".to_owned(),
            to: "small".to_owned(),
            reason: "r".to_owned(),
        },
    ));
    budget_3_small_masks_more(&mut events);
    events.push(sup::known(
        u64::try_from(events.len()).unwrap_or(0).saturating_add(1),
        anybot_protocol::KnownTaskEventKind::ModelSwapped {
            from: "small".to_owned(),
            to: "big".to_owned(),
            reason: "r".to_owned(),
        },
    ));
    budget_3_swap_back_restores_ten(&steps, events);
}

fn budget_3_input(
    steps: Vec<StepRecord>,
    usable: u64,
    events: Vec<anybot_protocol::TaskEvent>,
) -> BuildInput {
    let turns = vec![sup::record("u1", "hello")];
    let mut input = with_turns(steps, usable, events, Vec::new(), turns);
    input.tool_specs = vec![sup::tool_spec("t1")];
    input
}

fn budget_3_small_masks_more(events: &mut Vec<anybot_protocol::TaskEvent>) {
    let steps: Vec<StepRecord> = (0..30)
        .map(|i| sup::step_len(i, 1_500, &format!("r{i}")))
        .collect();
    let small = budget_3_input(steps.clone(), 16_000, events.clone());
    match compile(&small).unwrap() {
        Compiled::Sent {
            request,
            manifest,
            events: new,
            ..
        } => {
            let masked = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultMasked)
                .count();
            let fulls = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultFull)
                .count();
            assert!(masked > fulls);
            assert!(manifest.budget.estimate_calibrated <= small.model.usable());
            assert!(
                request
                    .blocks
                    .iter()
                    .filter(|b| b.kind == BlockKind::ToolIntent)
                    .count()
                    >= steps.len()
            );
            assert!(
                request.blocks.iter().any(|b| b.kind == BlockKind::Criterion
                    || b.kind == BlockKind::UserText
                    || b.kind == BlockKind::ToolDoc),
                "context-must items present on the small model"
            );
            sup::push_new(events, &new);
        }
        Compiled::Insufficient(_) => panic!("small model should fit after masking"),
    }
}

fn budget_3_swap_back_restores_ten(steps: &[StepRecord], events: Vec<anybot_protocol::TaskEvent>) {
    let big = budget_3_input(steps.to_vec(), 128_000, events);
    match compile(&big).unwrap() {
        Compiled::Sent { request, .. } => {
            let fulls: Vec<String> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultFull)
                .flat_map(|b| b.source_ids.clone())
                .collect();
            let recent: Vec<String> = steps
                .iter()
                .rev()
                .take(K114_1_RECENT_STEPS)
                .map(|s| format!("result-{}", s.source_ref))
                .collect();
            for id in recent {
                assert!(fulls.contains(&id), "recent steps verbatim after swap back");
            }
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn budget_drops_oldest_steps_first() {
    let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
    let mut collapses: Vec<usize> = Vec::new();
    for i in 0..40usize {
        let steps: Vec<StepRecord> = (0..=i)
            .map(|n| sup::step_len(n, 1_500, &format!("r{n}")))
            .collect();
        let input = assemble_input(steps, 16_000, events.clone(), Vec::new());
        match compile(&input).unwrap() {
            Compiled::Sent { events: new, .. } => {
                let count = new
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            NewEvent::Collapsed {
                                reason: crate::output::CollapseReason::Budget,
                                ..
                            }
                        )
                    })
                    .count();
                if count > 0 {
                    collapses.push(i);
                }
                sup::push_new(&mut events, &new);
            }
            Compiled::Insufficient(_) => panic!("40-step run must keep sending"),
        }
    }
    assert!(collapses.len() < 13, "drops must be far fewer than steps");
    for pair in collapses.windows(2) {
        let a = pair.first().copied().unwrap_or(0);
        let b = pair.get(1).copied().unwrap_or(0);
        assert!(
            b.saturating_sub(a) >= 3,
            "at least three steps between budget drops"
        );
    }
}

#[test]
fn prior_chat_keeps_recent_and_records_budget_drop() {
    let prior: Vec<crate::input::ChatRecord> = (0..400)
        .map(|i| {
            sup::record(
                &format!("p{i}"),
                &format!("old line {i} {}", "z".repeat(180)),
            )
        })
        .collect();
    let input = assemble_input(Vec::new(), 16_000, Vec::new(), prior);
    match compile(&input).unwrap() {
        Compiled::Sent {
            request, manifest, ..
        } => {
            let fulls = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::PriorChatFull)
                .count();
            let refs = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::PriorChatRef)
                .count();
            assert!(fulls > 0);
            assert_eq!(refs, 1);
            assert!(
                manifest
                    .excluded
                    .iter()
                    .any(|e| e.reason == crate::output::ExcludedReason::Budget)
            );
            assert!(manifest.excluded.len() > 100);
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn watermark_gate_keeps_shrinking_after_first_item() {
    let steps: Vec<StepRecord> = (0..16)
        .map(|i| sup::step_len(i, 2_000, &format!("r{i}")))
        .collect();
    let prior: Vec<crate::input::ChatRecord> = (0..30)
        .map(|i| sup::record(&format!("p{i}"), &"p".repeat(400)))
        .collect();
    let input = assemble_input(steps, 6_000, Vec::new(), prior);
    let usable = input.model.usable();
    let compiled = compile(&input).unwrap();
    if let Compiled::Sent {
        request, manifest, ..
    } = &compiled
    {
        let fulls = request
            .blocks
            .iter()
            .filter(|b| b.kind == BlockKind::PriorChatFull)
            .count();
        assert!(fulls > 0, "recent prior chat survives");
        assert!(
            manifest
                .excluded
                .iter()
                .any(|e| e.source_id.starts_with('p')
                    && e.reason == crate::output::ExcludedReason::Budget),
            "prior chat dropped in the second stage"
        );
        assert!(
            manifest.budget.estimate_calibrated
                <= crate::numbers::milli_of(usable, crate::numbers::K114_4_WATERMARK_MILLI),
            "estimate lands under the watermark"
        );
    } else {
        panic!("expected sent");
    }
}
